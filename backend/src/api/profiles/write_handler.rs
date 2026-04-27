type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::common::{error_response, ErrorSpec};

use super::{full_profile_response, parse_tag_uuids, sync_profile_tags};
use crate::api::{
    extract_filename,
    state::{CreateProfileBody, DataResponse, SuccessResponse, UpdateProfileBody},
};
use crate::db;
use crate::db::models::profiles::{NewProfile, Profile};
use crate::db::schema::profiles;

#[path = "write_service.rs"]
mod write_service;
use write_service::{
    build_update_changeset, load_and_verify_profile, non_empty_or_null, resolve_picture_filename,
    validate_and_prepare_update, validate_create,
};

fn build_create_model(
    user: &crate::db::models::users::User,
    payload: &CreateProfileBody,
    profile_picture: Option<String>,
) -> (NewProfile, Uuid) {
    let now = Utc::now();
    let profile_id = Uuid::new_v4();
    let images_json = payload.images.as_ref().and_then(|imgs| {
        serde_json::to_value(imgs.iter().map(|s| extract_filename(s)).collect::<Vec<_>>()).ok()
    });

    let model = NewProfile {
        id: profile_id,
        user_id: user.id,
        name: payload.name.trim().to_string(),
        bio: payload.bio.clone(),
        status_text: payload.status.as_deref().and_then(non_empty_or_null),
        status_emoji: None,
        // 24h TTL for legacy profile-create-with-status path; matches
        // the dedicated /profiles/me/status endpoint behavior.
        status_expires_at: payload
            .status
            .as_deref()
            .and_then(non_empty_or_null)
            .map(|_| now + chrono::Duration::hours(24)),
        profile_picture,
        images: images_json,
        program: payload.program.clone(),
        gradient_start: payload.gradient_start.clone(),
        gradient_end: payload.gradient_end.clone(),
        created_at: now,
        updated_at: now,
    };
    (model, profile_id)
}

async fn insert_profile(
    conn: &mut AsyncPgConnection,
    new_profile: &NewProfile,
    profile_id: Uuid,
    payload: &CreateProfileBody,
) -> Result<Profile> {
    let inserted = diesel::insert_into(profiles::table)
        .values(new_profile)
        .get_result::<Profile>(conn)
        .await?;

    let tag_ids = parse_tag_uuids(payload.tags.clone().or_else(|| payload.tag_ids.clone()));
    if !tag_ids.is_empty() {
        sync_profile_tags(conn, profile_id, &tag_ids).await?;
    }

    Ok(inserted)
}

pub(in crate::api) async fn profile_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateProfileBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let user_clone = user.clone();
    let payload_clone = payload.clone();
    let headers_clone = headers.clone();

    let result: Result<Response> = db::with_viewer_tx(viewer, move |conn| {
        async move {
            // validate_create re-verifies auth (cheap, uses cache) but we want
            // the existence check to run inside this viewer transaction.
            let user = match validate_create(conn, &headers_clone, &payload_clone).await {
                Ok(u) => u,
                Err(response) => return Ok::<Response, diesel::result::Error>(*response),
            };

            // Bio moderation runs AFTER validate_create so unauthenticated,
            // unauthorized, or duplicate-profile requests still get their
            // existing 401/403/409 instead of being masked by a 422 — and
            // we don't spend inference CPU on requests that would be
            // rejected cheaply either way.
            if let Some(ref bio) = payload_clone.bio {
                match moderate_profile_text(bio, ProfileTextField::Bio, &headers_clone).await {
                    Ok(None) => {}
                    Ok(Some(rejection)) => return Ok(rejection),
                    Err(error) => {
                        tracing::error!(%error, "bio moderation failed; rolling back create");
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }
            }
            if let Some(ref status) = payload_clone.status {
                match moderate_profile_text(status, ProfileTextField::Status, &headers_clone).await
                {
                    Ok(None) => {}
                    Ok(Some(rejection)) => return Ok(rejection),
                    Err(error) => {
                        tracing::error!(%error, "status moderation failed; rolling back create");
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }
            }

            let picture = match resolve_picture_filename(
                conn,
                &headers_clone,
                None,
                payload_clone.profile_picture.as_deref(),
            )
            .await
            {
                Ok(p) => p,
                Err(response) => return Ok(*response),
            };

            let (new_profile, profile_id) = build_create_model(&user, &payload_clone, picture);
            let inserted = insert_profile(conn, &new_profile, profile_id, &payload_clone)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let data = full_profile_response(conn, &inserted, &user.pid, Some(user.id))
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok((axum::http::StatusCode::CREATED, Json(DataResponse { data })).into_response())
        }
        .scope_boxed()
    })
    .await
    .map_err(Into::into);

    let _ = user_clone;
    result
}

async fn apply_update(
    conn: &mut AsyncPgConnection,
    profile: &Profile,
    payload: &UpdateProfileBody,
    changeset: crate::db::models::profiles::ProfileChangeset,
) -> Result<Profile> {
    let updated = diesel::update(profiles::table.find(profile.id))
        .set(&changeset)
        .get_result::<Profile>(conn)
        .await?;

    if payload.tags.is_some() || payload.tag_ids.is_some() {
        let resolved = parse_tag_uuids(payload.tags.clone().or_else(|| payload.tag_ids.clone()));
        sync_profile_tags(conn, profile.id, &resolved).await?;
    }

    Ok(updated)
}

pub(in crate::api) async fn profile_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProfileBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let headers_clone = headers.clone();
    let payload_clone = payload.clone();

    db::with_viewer_tx(viewer, move |conn| {
        async move {
            let (profile, user, picture) = match validate_and_prepare_update(
                conn,
                &headers_clone,
                &id,
                &payload_clone,
            )
            .await
            {
                Ok(data) => data,
                Err(response) => {
                    return Ok::<Response, diesel::result::Error>(*response);
                }
            };

            // Bio moderation runs AFTER validate_and_prepare_update so
            // unauthorised-profile and not-found requests still get 403/
            // 404 instead of a misleading 422, and we don't burn inference
            // CPU on them.
            if let Some(ref bio) = payload_clone.bio {
                match moderate_profile_text(bio, ProfileTextField::Bio, &headers_clone).await {
                    Ok(None) => {}
                    Ok(Some(rejection)) => return Ok(rejection),
                    Err(error) => {
                        tracing::error!(%error, "bio moderation failed; rolling back update");
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }
            }
            if let Some(ref status) = payload_clone.status {
                match moderate_profile_text(status, ProfileTextField::Status, &headers_clone).await
                {
                    Ok(None) => {}
                    Ok(Some(rejection)) => return Ok(rejection),
                    Err(error) => {
                        tracing::error!(%error, "status moderation failed; rolling back update");
                        return Err(diesel::result::Error::RollbackTransaction);
                    }
                }
            }

            let changeset = build_update_changeset(&payload_clone, picture);
            let updated = apply_update(conn, &profile, &payload_clone, changeset)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            let data = full_profile_response(conn, &updated, &user.pid, Some(user.id))
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Json(DataResponse { data }).into_response())
        }
        .scope_boxed()
    })
    .await
    .map_err(Into::into)
}

#[derive(Clone, Copy)]
enum ProfileTextField {
    Bio,
    Status,
}

impl ProfileTextField {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Bio => "bio",
            Self::Status => "status",
        }
    }

    const fn rejection_code(self) -> &'static str {
        match self {
            Self::Bio => "BIO_CONTENT_REJECTED",
            Self::Status => "STATUS_CONTENT_REJECTED",
        }
    }
}

/// Run the moderation engine on a profile free-text field (bio / status).
/// Returns:
/// - `Ok(None)` when moderation is disabled, the engine allows, or only
///   flags for review (flag is logged, publish proceeds).
/// - `Ok(Some(response))` when the engine blocks — caller must return it
///   as the handler's final response.
/// - `Err(_)` on an unexpected infrastructure error (inference panic,
///   spawn failure). Hard errors surface as 500; we never fall through to
///   "allow" on failure, because that would defeat the gate.
async fn moderate_profile_text(
    text: &str,
    field: ProfileTextField,
    headers: &HeaderMap,
) -> Result<Option<Response>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let Some(engine) = crate::moderation::shared() else {
        return Ok(None);
    };

    let owned = trimmed.to_string();
    let started = std::time::Instant::now();
    let scores = tokio::task::spawn_blocking(move || engine.score(&owned))
        .await
        .map_err(|e| crate::error::AppError::Message(format!("moderation task: {e}")))?
        .map_err(|e| crate::error::AppError::Message(format!("moderation inference: {e}")))?;
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    let thresholds = crate::moderation::Thresholds::BIO;
    let verdict = scores.verdict(&thresholds);
    let flagged = scores.flagged(&thresholds);

    let kind = field.as_str();
    metrics::histogram!("moderation_inference_latency_ms", "kind" => kind).record(elapsed_ms);
    metrics::counter!(
        "moderation_verdicts_total",
        "kind" => kind,
        "verdict" => verdict.as_str()
    )
    .increment(1);

    match verdict {
        crate::moderation::Verdict::Allow => Ok(None),
        crate::moderation::Verdict::Flag => {
            // Flag without block is rare with the BIO preset (flag and
            // block thresholds are equal), but we handle it defensively so
            // a future threshold tweak doesn't silently drop flagged input.
            tracing::warn!(
                field = kind,
                flagged = ?flagged.iter().map(|(c, s)| format!("{}={s:.2}", c.as_str())).collect::<Vec<_>>(),
                elapsed_ms,
                "profile moderation: flagged for review (allowed to publish)"
            );
            Ok(None)
        }
        crate::moderation::Verdict::Block => {
            let categories: Vec<&'static str> = flagged.iter().map(|(c, _)| c.as_str()).collect();
            tracing::warn!(
                field = kind,
                categories = ?categories,
                elapsed_ms,
                "profile moderation: blocked on publish"
            );
            let error_msg = rejection_message(field, &flagged);
            Ok(Some(error_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                headers,
                ErrorSpec {
                    error: error_msg,
                    code: field.rejection_code(),
                    details: Some(serde_json::json!({
                        "field": kind,
                        "categories": categories,
                    })),
                },
            )))
        }
    }
}

/// Build a user-facing rejection message. Polish-first (this is a Polish
/// product), falls back to a category listing. Wording is field-aware so
/// the snackbar reads naturally for either bio ("opis") or status.
fn rejection_message(
    field: ProfileTextField,
    flagged: &[(crate::moderation::Category, f32)],
) -> String {
    use crate::moderation::Category;
    let noun = match field {
        ProfileTextField::Bio => "opis",
        ProfileTextField::Status => "status",
    };
    if flagged.iter().any(|(c, _)| matches!(c, Category::SelfHarm)) {
        return format!(
            "Twój {noun} zawiera treści dotyczące samookaleczenia lub myśli samobójczych. \
             Jeśli potrzebujesz wsparcia, zadzwoń pod 116 123 (bezpłatny telefon zaufania). \
             Prosimy o edycję przed publikacją."
        );
    }
    let labels: Vec<&'static str> = flagged.iter().map(|(c, _)| c.as_str()).collect();
    format!(
        "Twój {noun} narusza zasady społeczności ({}). Proszę go zmienić przed zapisaniem.",
        labels.join(", ")
    )
}

pub(in crate::api) async fn profile_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let headers_clone = headers.clone();

    db::with_viewer_tx(viewer, move |conn| {
        async move {
            let (profile, _user) = match load_and_verify_profile(conn, &headers_clone, &id).await {
                Ok(p) => p,
                Err(response) => return Ok::<Response, diesel::result::Error>(*response),
            };

            diesel::delete(profiles::table.find(profile.id))
                .execute(conn)
                .await?;

            Ok(Json(SuccessResponse { success: true }).into_response())
        }
        .scope_boxed()
    })
    .await
    .map_err(Into::into)
}

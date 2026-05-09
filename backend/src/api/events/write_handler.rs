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
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::common::{error_response, ErrorSpec};
use crate::api::state::{
    AttendEventBody, AttendeeStatus, CreateEventBody, DataResponse, EventResponse, SuccessResponse,
    UpdateEventBody,
};
use crate::db;
use crate::db::models::events::{Event, EventChangeset, NewEvent};
use crate::jobs::enqueue_chat_membership_sync;

use super::events_interactions_repo::{
    delete_event_interaction_with_conn, upsert_event_interaction_with_conn,
    EVENT_INTERACTION_JOINED, EVENT_INTERACTION_SAVED,
};
use super::events_service::{
    forbidden, load_event_by_id, load_profile_for_user, not_found_event, parse_create_dates,
    profile_not_found, validate_max_attendees, validation_error,
};
use super::events_tags_repo::{sync_event_tags_with_conn, upsert_attendee_with_conn};
use super::events_tags_service::{
    maybe_sync_tags_with_conn, parse_event_tag_ids, resolve_event_tag_ids_with_conn,
};
use super::events_view::{build_event_response_raw, created_event_response, resolve_event_images};
use super::events_write_repo::{self, is_serialization_failure};
use super::events_write_service::{prepare_update_changeset, verify_event_cover_ownership};

/// Run Bielik-Guard against the supplied event text. Returns
/// `Ok(None)` when moderation passes (allow / flag) or is disabled,
/// `Ok(Some(response))` when the engine blocks — caller must return
/// it as the handler's final response. Mirrors `moderate_bio_text`
/// with the same `BIO` preset (conservative for sync gates) and a
/// distinct error code so the client can decide which form field
/// to highlight.
async fn moderate_event_text(
    text: &str,
    field: &'static str,
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

    metrics::histogram!("moderation_inference_latency_ms", "kind" => "event").record(elapsed_ms);
    metrics::counter!(
        "moderation_verdicts_total",
        "kind" => "event",
        "verdict" => verdict.as_str()
    )
    .increment(1);

    match verdict {
        crate::moderation::Verdict::Allow => Ok(None),
        crate::moderation::Verdict::Flag => {
            tracing::warn!(
                field = field,
                flagged = ?flagged.iter().map(|(c, s)| format!("{}={s:.2}", c.as_str())).collect::<Vec<_>>(),
                elapsed_ms,
                "event moderation: flagged for review (allowed to publish)"
            );
            Ok(None)
        }
        crate::moderation::Verdict::Block => {
            let categories: Vec<&'static str> = flagged.iter().map(|(c, _)| c.as_str()).collect();
            tracing::warn!(
                field = field,
                categories = ?categories,
                elapsed_ms,
                "event moderation: blocked on publish"
            );
            let label = match field {
                "title" => "Tytuł",
                "description" => "Opis",
                _ => "Treść",
            };
            let error_msg = format!(
                "{label} wydarzenia narusza zasady społeczności ({}). Proszę go zmienić przed zapisaniem.",
                categories.join(", ")
            );
            Ok(Some(error_response(
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                headers,
                ErrorSpec {
                    error: error_msg,
                    code: "EVENT_CONTENT_REJECTED",
                    details: Some(serde_json::json!({
                        "field": field,
                        "categories": categories,
                    })),
                },
            )))
        }
    }
}

/// Wrap an `AppError` into a diesel-level error so it can propagate through
/// transaction boundaries. `Message` / `Validation` preserve their text for
/// the caller to surface; everything else rolls back generically.
fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

async fn auth_and_viewer(
    headers: &HeaderMap,
) -> std::result::Result<(db::DbViewer, crate::db::models::users::User), Box<Response>> {
    let (_session, user) = crate::api::state::require_auth_db(headers).await?;
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    Ok((viewer, user))
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

struct ValidatedCreate {
    title: String,
    starts_at: chrono::DateTime<Utc>,
    ends_at: Option<chrono::DateTime<Utc>>,
    validated_tag_ids: Option<Vec<Uuid>>,
}

fn validate_create(
    headers: &HeaderMap,
    payload: &CreateEventBody,
) -> std::result::Result<ValidatedCreate, Box<Response>> {
    let (title, starts_at, ends_at) = parse_create_dates(headers, payload)?;
    validate_max_attendees(payload.max_attendees)
        .map_err(|msg| Box::new(validation_error(headers, msg)))?;
    let validated_tag_ids = match payload.tag_ids.clone() {
        Some(ids) => Some(parse_event_tag_ids(headers, ids)?),
        None => None,
    };
    Ok(ValidatedCreate {
        title,
        starts_at,
        ends_at,
        validated_tag_ids,
    })
}

fn build_new_event(
    profile_id: Uuid,
    validated: &ValidatedCreate,
    payload: &CreateEventBody,
) -> (NewEvent, Uuid) {
    let now = Utc::now();
    let event_id = Uuid::new_v4();
    let model = NewEvent {
        id: event_id,
        title: validated.title.clone(),
        description: payload.description.clone(),
        cover_image: payload.cover_image.clone(),
        category: payload.category.clone(),
        location: payload.location.clone(),
        starts_at: validated.starts_at,
        ends_at: validated.ends_at,
        creator_id: profile_id,
        conversation_id: None,
        latitude: payload.latitude,
        longitude: payload.longitude,
        max_attendees: payload.max_attendees,
        created_at: now,
        updated_at: now,
        requires_approval: payload.requires_approval.unwrap_or(false),
        visibility: normalize_visibility(payload.visibility.as_deref()),
    };
    (model, event_id)
}

fn normalize_visibility(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some("private") => "private".to_string(),
        _ => "public".to_string(),
    }
}

enum CreateOutcome {
    NoProfile,
    CoverInvalid(Box<Response>),
    Created { event: Box<Event>, profile_id: Uuid },
}

pub(in crate::api) async fn event_create(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Json(payload): Json<CreateEventBody>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };

    let validated = match validate_create(&headers, &payload) {
        Ok(v) => v,
        Err(response) => return Ok(*response),
    };

    // Sync moderation gate for user-supplied free-text fields, mirroring
    // the bio path. Title runs first because it's required; description
    // only runs if present + non-empty.
    if let Some(response) = moderate_event_text(&validated.title, "title", &headers).await? {
        return Ok(response);
    }
    if let Some(desc) = payload.description.as_deref() {
        if let Some(response) = moderate_event_text(desc, "description", &headers).await? {
            return Ok(response);
        }
    }

    let tag_names = payload.tags.clone();
    let user_id = viewer.user_id;
    let headers_for_tx = headers.clone();

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        let headers = headers_for_tx;
        let mut payload = payload;
        let validated = validated;
        let tag_names = tag_names;
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<CreateOutcome, diesel::result::Error>(CreateOutcome::NoProfile);
            };

            // Validate cover image ownership before INSERT so a creator
            // can't republish another user's upload as their event
            // cover. Skipped when no cover is supplied.
            if let Some(raw) = payload.cover_image.as_deref() {
                match verify_event_cover_ownership(conn, &headers, profile.id, raw).await {
                    Ok(filename) => payload.cover_image = Some(filename),
                    Err(response) => {
                        return Ok(CreateOutcome::CoverInvalid(response));
                    }
                }
            }

            let (new_event, event_id) = build_new_event(profile.id, &validated, &payload);
            let inserted = events_write_repo::insert_event_with_conn(conn, &new_event)
                .await
                .map_err(into_diesel)?;
            let tag_ids =
                resolve_event_tag_ids_with_conn(conn, tag_names, validated.validated_tag_ids)
                    .await
                    .map_err(into_diesel)?;
            if !tag_ids.is_empty() {
                sync_event_tags_with_conn(conn, event_id, &tag_ids)
                    .await
                    .map_err(into_diesel)?;
            }
            upsert_attendee_with_conn(conn, event_id, profile.id, ATTENDEE_GOING)
                .await
                .map_err(into_diesel)?;
            upsert_event_interaction_with_conn(
                conn,
                profile.id,
                event_id,
                EVENT_INTERACTION_JOINED,
            )
            .await
            .map_err(into_diesel)?;

            Ok(CreateOutcome::Created {
                event: Box::new(inserted),
                profile_id: profile.id,
            })
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from);

    let outcome = match outcome {
        Ok(o) => o,
        // Validation failures surfaced from inside the tx (e.g. tagIds that
        // reference missing tags) match the REST contract for this endpoint:
        // 400 BAD_REQUEST with code VALIDATION_ERROR, not 422. The default
        // `AppError::Validation` -> 422 mapping is wrong here.
        Err(crate::error::AppError::Validation(msg)) => {
            return Ok(validation_error(&headers, &msg));
        }
        Err(e) => return Err(e),
    };

    match outcome {
        CreateOutcome::NoProfile => Ok(profile_not_found(&headers)),
        CreateOutcome::CoverInvalid(response) => Ok(*response),
        CreateOutcome::Created { event, profile_id } => {
            let mut response = build_response_after_tx(viewer, &event, profile_id).await?;
            resolve_single(&mut response).await;
            Ok(created_event_response(response))
        }
    }
}

async fn build_response_after_tx(
    viewer: db::DbViewer,
    event: &Event,
    profile_id: Uuid,
) -> std::result::Result<EventResponse, crate::error::AppError> {
    let event = event.clone();
    db::with_viewer_tx(viewer, move |conn| {
        async move {
            build_event_response_raw(conn, &event, &profile_id)
                .await
                .map_err(into_diesel)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)
}

async fn resolve_single(response: &mut EventResponse) {
    let slice = std::slice::from_mut(response);
    resolve_event_images(slice).await;
}

// ---------------------------------------------------------------------------
// Update (serializable with retry)
// ---------------------------------------------------------------------------

enum UpdateOutcome {
    NoProfile,
    NotFound,
    Forbidden,
    Invalid(Box<Response>),
    Updated {
        event: Box<Event>,
        profile_id: Uuid,
        auto_approved: Vec<Uuid>,
    },
}

pub(in crate::api) async fn event_update(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateEventBody>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };

    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let validated_tag_ids = match payload.tag_ids.clone() {
        Some(ids) => match parse_event_tag_ids(&headers, ids) {
            Ok(parsed) => Some(parsed),
            Err(response) => return Ok(*response),
        },
        None => None,
    };

    // Sync moderation gate. Only run on fields the caller is actually
    // changing — partial updates omit unchanged fields.
    if let Some(ref new_title) = payload.title {
        if let Some(response) = moderate_event_text(new_title, "title", &headers).await? {
            return Ok(response);
        }
    }
    if let Some(Some(ref new_desc)) = payload.description {
        if let Some(response) = moderate_event_text(new_desc, "description", &headers).await? {
            return Ok(response);
        }
    }

    let mut conn = crate::db::conn().await?;
    let user_id = viewer.user_id;
    let mut attempts = 0;
    let outcome = loop {
        attempts += 1;
        let headers = &headers;
        let payload = &payload;
        let tag_names = payload.tags.clone();
        let validated_tag_ids_clone = validated_tag_ids.clone();
        let result = conn
            .build_transaction()
            .serializable()
            .run(|conn| {
                Box::pin(async move {
                    db::set_viewer_context(conn, viewer).await?;

                    let Some(profile) = load_profile_for_user(conn, user_id)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok::<UpdateOutcome, diesel::result::Error>(
                            UpdateOutcome::NoProfile,
                        );
                    };

                    let Some(event) = load_event_by_id(conn, event_uuid)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok(UpdateOutcome::NotFound);
                    };

                    if event.creator_id != profile.id {
                        return Ok(UpdateOutcome::Forbidden);
                    }

                    // Verify cover image ownership (if being set) before
                    // prepare_update_changeset, so a creator can't swap
                    // their event's cover to another user's upload. Clone
                    // payload locally so the verified filename can replace
                    // the raw one (extract_filename strips path noise).
                    let mut payload_local = payload.clone();
                    if let Some(Some(raw)) = payload_local.cover_image.as_ref() {
                        match verify_event_cover_ownership(conn, headers, profile.id, raw).await {
                            Ok(filename) => {
                                payload_local.cover_image = Some(Some(filename));
                            }
                            Err(response) => return Ok(UpdateOutcome::Invalid(response)),
                        }
                    }

                    let changeset: EventChangeset =
                        match prepare_update_changeset(headers, &event, profile.id, &payload_local)
                        {
                            Ok(c) => c,
                            Err(response) => return Ok(UpdateOutcome::Invalid(response)),
                        };

                    let sets_approval_false = changeset.requires_approval == Some(false);
                    let was_requiring = sets_approval_false
                        && crate::db::schema::events::table
                            .find(event_uuid)
                            .select(crate::db::schema::events::requires_approval)
                            .first::<bool>(conn)
                            .await?;

                    let updated =
                        events_write_repo::update_event_with_conn(conn, event_uuid, &changeset)
                            .await
                            .map_err(into_diesel)?;

                    let auto_approved = if was_requiring {
                        events_write_repo::auto_approve_pending_with_conn(
                            conn,
                            event_uuid,
                            updated.max_attendees,
                        )
                        .await
                        .map_err(into_diesel)?
                    } else {
                        vec![]
                    };

                    maybe_sync_tags_with_conn(conn, event_uuid, tag_names, validated_tag_ids_clone)
                        .await
                        .map_err(into_diesel)?;

                    Ok(UpdateOutcome::Updated {
                        event: Box::new(updated),
                        profile_id: profile.id,
                        auto_approved,
                    })
                })
            })
            .await;
        match result {
            Ok(val) => break val,
            Err(ref e)
                if attempts < events_write_repo::MAX_ATTEMPTS && is_serialization_failure(e) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(10u64 << attempts)).await;
            }
            Err(e) => {
                // Validation failures (e.g. tagIds) match the existing REST
                // contract: 400 BAD_REQUEST, not the default 422 mapping for
                // AppError::Validation.
                let app_err = crate::error::AppError::from(e);
                if let crate::error::AppError::Validation(msg) = app_err {
                    return Ok(validation_error(headers, &msg));
                }
                return Err(app_err);
            }
        }
    };

    match outcome {
        UpdateOutcome::NoProfile => Ok(profile_not_found(&headers)),
        UpdateOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        UpdateOutcome::Forbidden => Ok(forbidden(
            &headers,
            "Only the creator can update this event",
        )),
        UpdateOutcome::Invalid(response) => Ok(*response),
        UpdateOutcome::Updated {
            event,
            profile_id,
            auto_approved,
        } => {
            for pid in &auto_approved {
                if let Err(error) = enqueue_chat_membership_sync(&event.id, pid, false).await {
                    tracing::warn!(
                        %error,
                        event_id = %event.id,
                        profile_id = %pid,
                        "failed to enqueue chat membership sync for auto-approved attendee"
                    );
                }
            }
            let mut response = build_response_after_tx(viewer, &event, profile_id).await?;
            resolve_single(&mut response).await;
            Ok(Json(DataResponse { data: response }).into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

enum SimpleOutcome {
    NoProfile,
    NotFound,
    Forbidden,
    Ok,
}

pub(in crate::api) async fn event_delete(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let user_id = viewer.user_id;

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<SimpleOutcome, diesel::result::Error>(SimpleOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(SimpleOutcome::NotFound);
            };
            if event.creator_id != profile.id {
                return Ok(SimpleOutcome::Forbidden);
            }
            events_write_repo::delete_event_with_conn(conn, event_uuid)
                .await
                .map_err(into_diesel)?;
            Ok(SimpleOutcome::Ok)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        SimpleOutcome::NoProfile => Ok(profile_not_found(&headers)),
        SimpleOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        SimpleOutcome::Forbidden => Ok(forbidden(
            &headers,
            "Only the creator can delete this event",
        )),
        SimpleOutcome::Ok => Ok(Json(DataResponse {
            data: SuccessResponse { success: true },
        })
        .into_response()),
    }
}

// ---------------------------------------------------------------------------
// Attend (serializable)
// ---------------------------------------------------------------------------

const ATTENDEE_GOING: &str = "going";
const ATTENDEE_INTERESTED: &str = "interested";
const ATTENDEE_INVITED: &str = "invited";

fn resolve_attend_status(payload: Option<Json<AttendEventBody>>) -> &'static str {
    match payload
        .and_then(|Json(body)| body.status)
        .unwrap_or(AttendeeStatus::Going)
    {
        AttendeeStatus::Going | AttendeeStatus::Pending => ATTENDEE_GOING,
        AttendeeStatus::Interested => ATTENDEE_INTERESTED,
        AttendeeStatus::Invited => ATTENDEE_INVITED,
    }
}

enum AttendOutcome {
    NoProfile,
    NotFound,
    Full,
    Accepted {
        event: Box<Event>,
        profile_id: Uuid,
        written_status: String,
    },
}

pub(in crate::api) async fn event_attend(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Option<Json<AttendEventBody>>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let status_str = resolve_attend_status(payload);
    let user_id = viewer.user_id;

    let mut conn = crate::db::conn().await?;
    let mut attempts = 0;
    let outcome = loop {
        attempts += 1;
        let result: std::result::Result<AttendOutcome, diesel::result::Error> = conn
            .build_transaction()
            .serializable()
            .run(|conn| {
                Box::pin(async move {
                    db::set_viewer_context(conn, viewer).await?;

                    let Some(profile) = load_profile_for_user(conn, user_id)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok(AttendOutcome::NoProfile);
                    };
                    let Some(event) = load_event_by_id(conn, event_uuid)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok(AttendOutcome::NotFound);
                    };

                    let requires_approval = event.requires_approval
                        && status_str == ATTENDEE_GOING
                        && event.creator_id != profile.id;

                    let outcome = events_write_repo::check_capacity_and_upsert_with_conn(
                        conn,
                        event_uuid,
                        profile.id,
                        status_str,
                        event.max_attendees,
                        None,
                        requires_approval,
                    )
                    .await?;

                    match outcome {
                        events_write_repo::UpsertOutcome::Full => Ok(AttendOutcome::Full),
                        events_write_repo::UpsertOutcome::StatusMismatch => {
                            debug_assert!(
                                false,
                                "StatusMismatch returned with require_status = None"
                            );
                            Ok(AttendOutcome::Full)
                        }
                        events_write_repo::UpsertOutcome::Accepted(s) => {
                            Ok(AttendOutcome::Accepted {
                                event: Box::new(event),
                                profile_id: profile.id,
                                written_status: s,
                            })
                        }
                    }
                })
            })
            .await;
        match result {
            Ok(val) => break val,
            Err(ref e)
                if attempts < events_write_repo::MAX_ATTEMPTS && is_serialization_failure(e) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(10u64 << attempts)).await;
            }
            Err(e) => return Err(crate::error::AppError::from(e)),
        }
    };

    match outcome {
        AttendOutcome::NoProfile => Ok(profile_not_found(&headers)),
        AttendOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        AttendOutcome::Full => Ok(validation_error(&headers, "Event is full")),
        AttendOutcome::Accepted {
            event,
            profile_id,
            written_status,
        } => {
            if written_status == ATTENDEE_GOING {
                if let Err(error) =
                    enqueue_chat_membership_sync(&event.id, &profile_id, false).await
                {
                    tracing::warn!(
                        %error,
                        event_id = %event.id,
                        profile_id = %profile_id,
                        "failed to enqueue chat membership sync after attend"
                    );
                }
            }
            let mut response = build_response_after_tx(viewer, &event, profile_id).await?;
            resolve_single(&mut response).await;
            Ok(Json(DataResponse { data: response }).into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Save / Unsave (simple)
// ---------------------------------------------------------------------------

enum LoadedOutcome {
    NoProfile,
    NotFound,
    Loaded { event: Box<Event>, profile_id: Uuid },
}

pub(in crate::api) async fn event_save(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let user_id = viewer.user_id;
    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<LoadedOutcome, diesel::result::Error>(LoadedOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(LoadedOutcome::NotFound);
            };
            upsert_event_interaction_with_conn(
                conn,
                profile.id,
                event_uuid,
                EVENT_INTERACTION_SAVED,
            )
            .await
            .map_err(into_diesel)?;
            Ok(LoadedOutcome::Loaded {
                event: Box::new(event),
                profile_id: profile.id,
            })
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    finalize_loaded_event(&headers, &id, viewer, outcome).await
}

pub(in crate::api) async fn event_unsave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    let user_id = viewer.user_id;
    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<LoadedOutcome, diesel::result::Error>(LoadedOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(LoadedOutcome::NotFound);
            };
            delete_event_interaction_with_conn(
                conn,
                profile.id,
                event_uuid,
                EVENT_INTERACTION_SAVED,
            )
            .await
            .map_err(into_diesel)?;
            Ok(LoadedOutcome::Loaded {
                event: Box::new(event),
                profile_id: profile.id,
            })
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    finalize_loaded_event(&headers, &id, viewer, outcome).await
}

async fn finalize_loaded_event(
    headers: &HeaderMap,
    id: &str,
    viewer: db::DbViewer,
    outcome: LoadedOutcome,
) -> Result<Response> {
    match outcome {
        LoadedOutcome::NoProfile => Ok(profile_not_found(headers)),
        LoadedOutcome::NotFound => Ok(not_found_event(headers, id)),
        LoadedOutcome::Loaded { event, profile_id } => {
            let mut response = build_response_after_tx(viewer, &event, profile_id).await?;
            resolve_single(&mut response).await;
            Ok(Json(DataResponse { data: response }).into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Approve / Reject attendee
// ---------------------------------------------------------------------------

enum ApproveOutcome {
    NoProfile,
    NotFound,
    Forbidden,
    Full,
    StatusMismatch,
    Accepted { event_id: Uuid, title: String },
}

pub(in crate::api) async fn event_approve_attendee(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path((id, profile_id_str)): Path<(String, String)>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let target_profile_id =
        match crate::api::parse_uuid_response(&profile_id_str, "profile", &headers) {
            Ok(uuid) => uuid,
            Err(response) => return Ok(*response),
        };
    let user_id = viewer.user_id;

    let mut conn = crate::db::conn().await?;
    let mut attempts = 0;
    let outcome = loop {
        attempts += 1;
        let result: std::result::Result<ApproveOutcome, diesel::result::Error> = conn
            .build_transaction()
            .serializable()
            .run(|conn| {
                Box::pin(async move {
                    db::set_viewer_context(conn, viewer).await?;

                    let Some(profile) = load_profile_for_user(conn, user_id)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok(ApproveOutcome::NoProfile);
                    };
                    let Some(event) = load_event_by_id(conn, event_uuid)
                        .await
                        .map_err(into_diesel)?
                    else {
                        return Ok(ApproveOutcome::NotFound);
                    };
                    if event.creator_id != profile.id {
                        return Ok(ApproveOutcome::Forbidden);
                    }

                    let outcome = events_write_repo::check_capacity_and_upsert_with_conn(
                        conn,
                        event_uuid,
                        target_profile_id,
                        "going",
                        event.max_attendees,
                        Some("pending"),
                        false,
                    )
                    .await?;
                    match outcome {
                        events_write_repo::UpsertOutcome::Full => Ok(ApproveOutcome::Full),
                        events_write_repo::UpsertOutcome::StatusMismatch => {
                            Ok(ApproveOutcome::StatusMismatch)
                        }
                        events_write_repo::UpsertOutcome::Accepted(_) => {
                            Ok(ApproveOutcome::Accepted {
                                event_id: event.id,
                                title: event.title,
                            })
                        }
                    }
                })
            })
            .await;
        match result {
            Ok(val) => break val,
            Err(ref e)
                if attempts < events_write_repo::MAX_ATTEMPTS && is_serialization_failure(e) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(10u64 << attempts)).await;
            }
            Err(e) => return Err(crate::error::AppError::from(e)),
        }
    };

    match outcome {
        ApproveOutcome::NoProfile => Ok(profile_not_found(&headers)),
        ApproveOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        ApproveOutcome::Forbidden => Ok(forbidden(
            &headers,
            "Only the creator can approve attendees",
        )),
        ApproveOutcome::Full => Ok(validation_error(&headers, "Event is full")),
        ApproveOutcome::StatusMismatch => Ok(validation_error(
            &headers,
            "Attendee is not pending approval",
        )),
        ApproveOutcome::Accepted { event_id, title } => {
            if let Err(error) =
                enqueue_chat_membership_sync(&event_id, &target_profile_id, false).await
            {
                tracing::warn!(
                    %error,
                    %event_id,
                    profile_id = %target_profile_id,
                    "failed to enqueue chat membership sync after approve"
                );
            }
            tokio::spawn(async move {
                notify_event_approval(target_profile_id, &title, true).await;
            });
            Ok(Json(DataResponse {
                data: SuccessResponse { success: true },
            })
            .into_response())
        }
    }
}

enum RejectOutcome {
    NoProfile,
    NotFound,
    Forbidden,
    NotPending,
    Rejected { title: String },
}

pub(in crate::api) async fn event_reject_attendee(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path((id, profile_id_str)): Path<(String, String)>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let target_profile_id =
        match crate::api::parse_uuid_response(&profile_id_str, "profile", &headers) {
            Ok(uuid) => uuid,
            Err(response) => return Ok(*response),
        };
    let user_id = viewer.user_id;

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<RejectOutcome, diesel::result::Error>(RejectOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(RejectOutcome::NotFound);
            };
            if event.creator_id != profile.id {
                return Ok(RejectOutcome::Forbidden);
            }
            let deleted = events_write_repo::delete_pending_attendee_with_conn(
                conn,
                event_uuid,
                target_profile_id,
            )
            .await
            .map_err(into_diesel)?;
            if !deleted {
                return Ok(RejectOutcome::NotPending);
            }
            Ok(RejectOutcome::Rejected { title: event.title })
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        RejectOutcome::NoProfile => Ok(profile_not_found(&headers)),
        RejectOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        RejectOutcome::Forbidden => {
            Ok(forbidden(&headers, "Only the creator can reject attendees"))
        }
        RejectOutcome::NotPending => Ok(validation_error(
            &headers,
            "Attendee is not pending approval",
        )),
        RejectOutcome::Rejected { title } => {
            tokio::spawn(async move {
                notify_event_approval(target_profile_id, &title, false).await;
            });
            Ok(Json(DataResponse {
                data: SuccessResponse { success: true },
            })
            .into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Leave
// ---------------------------------------------------------------------------

enum LeaveOutcome {
    NoProfile,
    NotFound,
    CreatorCannotLeave,
    Left { event: Box<Event>, profile_id: Uuid },
}

pub(in crate::api) async fn event_leave(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (viewer, _user) = match auth_and_viewer(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let user_id = viewer.user_id;

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<LeaveOutcome, diesel::result::Error>(LeaveOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(LeaveOutcome::NotFound);
            };
            if event.creator_id == profile.id {
                return Ok(LeaveOutcome::CreatorCannotLeave);
            }
            events_write_repo::delete_event_attendee_with_conn(conn, event_uuid, profile.id)
                .await
                .map_err(into_diesel)?;
            delete_event_interaction_with_conn(
                conn,
                profile.id,
                event_uuid,
                EVENT_INTERACTION_JOINED,
            )
            .await
            .map_err(into_diesel)?;
            Ok(LeaveOutcome::Left {
                event: Box::new(event),
                profile_id: profile.id,
            })
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        LeaveOutcome::NoProfile => Ok(profile_not_found(&headers)),
        LeaveOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        LeaveOutcome::CreatorCannotLeave => {
            Ok(forbidden(&headers, "Event creator cannot leave the event"))
        }
        LeaveOutcome::Left { event, profile_id } => {
            if let Err(error) = enqueue_chat_membership_sync(&event.id, &profile_id, true).await {
                tracing::warn!(
                    %error,
                    event_id = %event.id,
                    profile_id = %profile_id,
                    "failed to enqueue chat membership sync after leave"
                );
            }
            let mut response = build_response_after_tx(viewer, &event, profile_id).await?;
            resolve_single(&mut response).await;
            Ok(Json(DataResponse { data: response }).into_response())
        }
    }
}

// ---------------------------------------------------------------------------
// Push notification for approval/rejection (spawned background task)
// ---------------------------------------------------------------------------

async fn notify_event_approval(target_profile_id: Uuid, event_title: &str, approved: bool) {
    let Ok(mut conn) = crate::db::conn().await else {
        return;
    };
    // Resolve the owner user_id via a narrow SECURITY DEFINER helper so the
    // API role doesn't need broad SELECT on profiles.
    let Ok(Some(user_id)) = db::profile_owner_user_id(&mut conn, target_profile_id).await else {
        return;
    };

    let body = if approved {
        format!("Twoje zgloszenie na \"{event_title}\" zostalo zatwierdzone!")
    } else {
        format!("Twoje zgloszenie na \"{event_title}\" zostalo odrzucone.")
    };

    crate::api::chat::push::notify_push(vec![user_id], Uuid::nil(), 0, &body).await;
}

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
    build_update_changeset, load_and_verify_profile, resolve_picture_filename,
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

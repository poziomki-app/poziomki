#[path = "http.rs"]
mod profiles_http;
#[path = "repo.rs"]
mod profiles_repo;
#[path = "tags_repo.rs"]
mod profiles_tags_repo;
#[path = "tags_service.rs"]
mod profiles_tags_service;
#[path = "view.rs"]
mod profiles_view;
#[path = "write_handler.rs"]
mod profiles_write_handler;

type Result<T> = crate::error::AppResult<T>;

use super::state::DataResponse;
use crate::api::auth_or_respond;
use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use profiles_http::not_found_profile;
pub(super) use profiles_http::validation_error;
use profiles_repo::{load_profile_by_user_id, load_profile_with_owner_pid};
pub(super) use profiles_tags_repo::sync_profile_tags;
pub(super) use profiles_tags_service::parse_tag_uuids;
pub(super) use profiles_view::{full_profile_response, profile_to_response};
pub(super) use profiles_write_handler::{profile_create, profile_delete, profile_update};

pub(super) async fn profile_me(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let format = crate::api::image_format_from_headers(&headers);

    let profile = load_profile_by_user_id(user.id).await?;

    let data = match profile {
        Some(ref p) => Some(full_profile_response(p, &user.pid, Some(user.id), format).await?),
        None => None,
    };

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let format = crate::api::image_format_from_headers(&headers);

    let profile_uuid = super::parse_uuid(&id, "profile")?;

    let Some((profile, user_pid)) = load_profile_with_owner_pid(profile_uuid).await? else {
        return Ok(not_found_profile(&headers, &id));
    };

    let data = profile_to_response(&profile, &user_pid, Some(user.id), format).await;
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get_full(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let format = crate::api::image_format_from_headers(&headers);

    let profile_uuid = super::parse_uuid(&id, "profile")?;

    let Some((profile, user_pid)) = load_profile_with_owner_pid(profile_uuid).await? else {
        return Ok(not_found_profile(&headers, &id));
    };

    let data = full_profile_response(&profile, &user_pid, Some(user.id), format).await?;
    Ok(Json(DataResponse { data }).into_response())
}

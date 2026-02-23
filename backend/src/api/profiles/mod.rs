#[path = "http_support.rs"]
mod profiles_http_support;
#[path = "mutations.rs"]
mod profiles_mutations;
#[path = "repo.rs"]
mod profiles_repo;
#[path = "tags.rs"]
mod profiles_tags;
#[path = "view.rs"]
mod profiles_view;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::state::{require_auth_db, DataResponse};
use profiles_http_support::not_found_profile;
pub(super) use profiles_http_support::validation_error;
pub(super) use profiles_mutations::{profile_create, profile_delete, profile_update};
use profiles_repo::{load_profile_by_user_id, load_profile_with_owner_pid};
pub(super) use profiles_tags::{parse_tag_uuids, sync_profile_tags};
pub(super) use profiles_view::{full_profile_response, profile_to_response};

pub(super) async fn profile_me(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile = load_profile_by_user_id(user.id).await?;

    let data = match profile {
        Some(ref p) => Some(full_profile_response(p, &user.pid).await?),
        None => None,
    };

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let Some((profile, user_pid)) = load_profile_with_owner_pid(profile_uuid).await? else {
        return Ok(not_found_profile(&headers, &id));
    };

    let data = profile_to_response(&profile, &user_pid).await;
    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get_full(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, _user) = match require_auth_db(&headers).await {
        Ok(auth) => auth,
        Err(response) => return Ok(*response),
    };

    let profile_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid profile ID".to_string()))?;

    let Some((profile, user_pid)) = load_profile_with_owner_pid(profile_uuid).await? else {
        return Ok(not_found_profile(&headers, &id));
    };

    let data = full_profile_response(&profile, &user_pid).await?;
    Ok(Json(DataResponse { data }).into_response())
}

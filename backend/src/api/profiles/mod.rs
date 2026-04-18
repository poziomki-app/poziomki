#[path = "blocks.rs"]
pub mod profiles_blocks;
#[path = "bookmarks.rs"]
mod profiles_bookmarks;
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
use crate::db;
use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use diesel_async::scoped_futures::ScopedFutureExt;
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
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let viewer_pid = user.pid;
    let user_id = user.id;

    let data = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let profile = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            let result = match profile {
                Some(ref p) => Some(
                    full_profile_response(conn, p, &viewer_pid, Some(user_id))
                        .await
                        .map_err(|_| diesel::result::Error::RollbackTransaction)?,
                ),
                None => None,
            };
            Ok::<_, diesel::result::Error>(result)
        }
        .scope_boxed()
    })
    .await?;

    Ok(Json(DataResponse { data }).into_response())
}

pub(super) async fn profile_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let profile_uuid = super::parse_uuid(&id, "profile")?;

    let result = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let loaded = load_profile_with_owner_pid(conn, profile_uuid)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            match loaded {
                Some((profile, owner_pid)) => {
                    let response =
                        profile_to_response(conn, &profile, &owner_pid, Some(user_id)).await;
                    Ok::<_, diesel::result::Error>(Some(response))
                }
                None => Ok(None),
            }
        }
        .scope_boxed()
    })
    .await?;

    result.map_or_else(
        || Ok(not_found_profile(&headers, &id)),
        |data| Ok(Json(DataResponse { data }).into_response()),
    )
}

pub(super) async fn profile_get_full(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let profile_uuid = super::parse_uuid(&id, "profile")?;

    let result = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some((profile, owner_pid)) = load_profile_with_owner_pid(conn, profile_uuid)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<_, diesel::result::Error>(None);
            };

            let mut data = full_profile_response(conn, &profile, &owner_pid, Some(user_id))
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            if let Some(my_profile) = load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            {
                data.is_bookmarked =
                    profiles_bookmarks::is_bookmarked(conn, my_profile.id, profile_uuid)
                        .await
                        .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            }

            Ok(Some(data))
        }
        .scope_boxed()
    })
    .await?;

    result.map_or_else(
        || Ok(not_found_profile(&headers, &id)),
        |data| Ok(Json(DataResponse { data }).into_response()),
    )
}

pub(super) async fn profile_bookmark_handler(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let target_uuid = super::parse_uuid(&id, "profile")?;
    let headers_clone = headers.clone();

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(my_profile) = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<Option<Response>, diesel::result::Error>(None);
            };
            let response = profiles_bookmarks::profile_bookmark(
                conn,
                &headers_clone,
                &my_profile,
                target_uuid,
            )
            .await
            .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Some(response))
        }
        .scope_boxed()
    })
    .await?;

    Ok(outcome.unwrap_or_else(|| not_found_profile(&headers, "me")))
}

pub(super) async fn profile_unbookmark_handler(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let target_uuid = super::parse_uuid(&id, "profile")?;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(my_profile) = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<Option<Response>, diesel::result::Error>(None);
            };
            let response = profiles_bookmarks::profile_unbookmark(conn, my_profile.id, target_uuid)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Some(response))
        }
        .scope_boxed()
    })
    .await?;

    Ok(outcome.unwrap_or_else(|| not_found_profile(&headers, "me")))
}

pub(super) async fn profiles_bookmarked_handler(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(my_profile) = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<Option<Vec<_>>, diesel::result::Error>(None);
            };
            let data = profiles_bookmarks::profiles_bookmarked(conn, my_profile.id, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Some(data))
        }
        .scope_boxed()
    })
    .await?;

    outcome.map_or_else(
        || Ok(not_found_profile(&headers, "me")),
        |data| Ok(Json(DataResponse { data }).into_response()),
    )
}

pub(super) async fn profile_block_handler(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let target_uuid = super::parse_uuid(&id, "profile")?;
    let headers_clone = headers.clone();

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(my_profile) = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<Option<Response>, diesel::result::Error>(None);
            };
            let response =
                profiles_blocks::profile_block(conn, &headers_clone, my_profile.id, target_uuid)
                    .await
                    .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Some(response))
        }
        .scope_boxed()
    })
    .await?;

    Ok(outcome.unwrap_or_else(|| not_found_profile(&headers, "me")))
}

pub(super) async fn profile_unblock_handler(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;
    let target_uuid = super::parse_uuid(&id, "profile")?;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(my_profile) = profiles_repo::load_profile_by_user_id(conn, user_id)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?
            else {
                return Ok::<Option<Response>, diesel::result::Error>(None);
            };
            let response = profiles_blocks::profile_unblock(conn, my_profile.id, target_uuid)
                .await
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;
            Ok(Some(response))
        }
        .scope_boxed()
    })
    .await?;

    Ok(outcome.unwrap_or_else(|| not_found_profile(&headers, "me")))
}

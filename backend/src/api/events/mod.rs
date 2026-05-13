#[path = "interactions_repo.rs"]
mod events_interactions_repo;
#[path = "repo.rs"]
mod events_repo;
#[path = "service.rs"]
mod events_service;
#[path = "tags_repo.rs"]
mod events_tags_repo;
#[path = "tags_service.rs"]
mod events_tags_service;
#[path = "view.rs"]
mod events_view;
#[path = "write_handler.rs"]
mod events_write_handler;
#[path = "write_repo.rs"]
mod events_write_repo;
#[path = "write_service.rs"]
mod events_write_service;
mod place_poll;
mod report_handler;
mod report_repo;

type Result<T> = crate::error::AppResult<T>;

use crate::app::AppContext;
use crate::db;
use axum::response::Response;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use diesel_async::scoped_futures::ScopedFutureExt;
use uuid::Uuid;

use super::state::{DataResponse, EventsQuery};
use events_service::{not_found_event, profile_not_found};

pub(super) use events_interactions_repo::EVENT_INTERACTION_SAVED;
pub(super) use events_view::{
    build_event_response_raw, build_event_responses_raw,
    resolve_event_images as resolve_event_images_for_responses,
};
pub(super) use events_write_handler::{
    event_approve_attendee, event_attend, event_create, event_delete, event_leave,
    event_reject_attendee, event_save, event_unsave, event_update,
};
pub(super) use place_poll::{place_poll_create, place_poll_get, place_poll_vote};
pub(super) use report_handler::event_report;

const PRIVATE_CACHE_SHORT: HeaderValue = HeaderValue::from_static("private, max-age=60");

fn with_private_short_cache(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(axum::http::header::CACHE_CONTROL, PRIVATE_CACHE_SHORT);
    response
}

enum ListOutcome {
    NoProfile,
    Listed(Vec<crate::api::state::EventResponse>),
}

async fn list_events_with_viewer<F>(
    headers: &HeaderMap,
    load: F,
) -> std::result::Result<Response, crate::error::AppError>
where
    F: for<'c> FnOnce(
            &'c mut diesel_async::AsyncPgConnection,
            uuid::Uuid,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = std::result::Result<
                            Vec<crate::db::models::events::Event>,
                            crate::error::AppError,
                        >,
                    > + Send
                    + 'c,
            >,
        > + Send,
{
    let (_session, user) = match crate::api::state::require_auth_db(headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = events_service::load_profile_for_user(conn, user.id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<ListOutcome, diesel::result::Error>(ListOutcome::NoProfile);
            };
            let events = load(conn, profile.id).await.map_err(into_diesel)?;
            let responses = events_view::build_event_responses_raw(conn, &events, &profile.id)
                .await
                .map_err(into_diesel)?;
            Ok(ListOutcome::Listed(responses))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        ListOutcome::NoProfile => Ok(profile_not_found(headers)),
        ListOutcome::Listed(mut responses) => {
            events_view::resolve_event_images(&mut responses).await;
            Ok(with_private_short_cache(
                Json(DataResponse { data: responses }).into_response(),
            ))
        }
    }
}

fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    diesel::result::Error::QueryBuilderError(Box::new(e))
}

pub(super) async fn events_list(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Response> {
    let limit = i64::from(query.limit.unwrap_or(20).clamp(1, 100));
    let now = Utc::now();
    list_events_with_viewer(&headers, move |conn, _profile_id| {
        Box::pin(async move { events_repo::list_upcoming_events(conn, now, limit).await })
    })
    .await
}

pub(super) async fn events_mine(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    list_events_with_viewer(&headers, |conn, profile_id| {
        Box::pin(async move { events_repo::list_events_by_creator(conn, profile_id).await })
    })
    .await
}

pub(super) async fn events_saved(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    list_events_with_viewer(&headers, |conn, profile_id| {
        Box::pin(async move { events_repo::list_saved_events(conn, profile_id).await })
    })
    .await
}

enum EventGetOutcome {
    NoProfile,
    NotFound,
    Loaded(Box<crate::api::state::EventResponse>),
}

pub(super) async fn event_get(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            let Some(profile) = events_service::load_profile_for_user(conn, user.id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<EventGetOutcome, diesel::result::Error>(EventGetOutcome::NoProfile);
            };

            let Some(event) = events_repo::find_event(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(EventGetOutcome::NotFound);
            };

            let response = build_event_response_raw(conn, &event, &profile.id)
                .await
                .map_err(into_diesel)?;
            Ok(EventGetOutcome::Loaded(Box::new(response)))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        EventGetOutcome::NoProfile => Ok(profile_not_found(&headers)),
        EventGetOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        EventGetOutcome::Loaded(response) => {
            let mut slice = [*response];
            events_view::resolve_event_images(&mut slice).await;
            let Some(response) = slice.into_iter().next() else {
                return Err(crate::error::AppError::Message(
                    "empty response".to_string(),
                ));
            };
            Ok(with_private_short_cache(
                Json(DataResponse { data: response }).into_response(),
            ))
        }
    }
}

enum AttendeesOutcome {
    NoProfile,
    NotFound,
    Listed(Vec<crate::api::state::AttendeeFullInfo>),
}

pub(super) async fn event_attendees(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };

    let event_uuid = Uuid::parse_str(&id)
        .map_err(|_| crate::error::AppError::Message("Invalid event ID".to_string()))?;

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };

    let outcome = db::with_viewer_tx(viewer, |conn| {
        async move {
            if events_service::load_profile_for_user(conn, user.id)
                .await
                .map_err(into_diesel)?
                .is_none()
            {
                return Ok::<AttendeesOutcome, diesel::result::Error>(AttendeesOutcome::NoProfile);
            }
            let Some(event) = events_repo::find_event(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(AttendeesOutcome::NotFound);
            };
            let list = events_view::attendee_info(conn, event_uuid, event.creator_id, user.id)
                .await
                .map_err(into_diesel)?;
            Ok(AttendeesOutcome::Listed(list))
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        AttendeesOutcome::NoProfile => Ok(profile_not_found(&headers)),
        AttendeesOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        AttendeesOutcome::Listed(list) => Ok(Json(DataResponse { data: list }).into_response()),
    }
}

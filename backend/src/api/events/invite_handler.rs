type Result<T> = crate::error::AppResult<T>;

use axum::response::Response;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::state::{InviteUsersBody, SuccessResponse};
use crate::app::AppContext;
use crate::db;
use crate::db::models::event_attendees::EventAttendee;
use crate::db::schema::event_attendees;

use super::events_service::{
    forbidden, load_event_by_id, load_profile_for_user, not_found_event, profile_not_found,
    validation_error,
};

const INVITED_STATUS: &str = "invited";

fn into_diesel(e: crate::error::AppError) -> diesel::result::Error {
    match e {
        crate::error::AppError::Message(_) | crate::error::AppError::Validation(_) => {
            diesel::result::Error::QueryBuilderError(Box::new(e))
        }
        crate::error::AppError::Any(_) => diesel::result::Error::RollbackTransaction,
    }
}

enum InviteOutcome {
    NoProfile,
    NotFound,
    NotCreator,
    Inserted,
}

enum UninviteOutcome {
    NoProfile,
    NotFound,
    NotCreator,
    Removed,
}

pub(in crate::api) async fn event_invite_users(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<InviteUsersBody>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };

    if body.profile_ids.is_empty() {
        return Ok(validation_error(&headers, "Lista zaproszonych jest pusta"));
    }
    let mut profile_uuids: Vec<Uuid> = Vec::with_capacity(body.profile_ids.len());
    for raw in &body.profile_ids {
        let Ok(uuid) = Uuid::parse_str(raw) else {
            return Ok(validation_error(
                &headers,
                "Nieprawidłowy identyfikator profilu",
            ));
        };
        profile_uuids.push(uuid);
    }

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<InviteOutcome, diesel::result::Error>(InviteOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(InviteOutcome::NotFound);
            };
            if event.creator_id != profile.id {
                return Ok(InviteOutcome::NotCreator);
            }

            let rows: Vec<EventAttendee> = profile_uuids
                .iter()
                .map(|pid| EventAttendee {
                    event_id: event_uuid,
                    profile_id: *pid,
                    status: INVITED_STATUS.to_string(),
                })
                .collect();

            diesel::insert_into(event_attendees::table)
                .values(&rows)
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;

            Ok(InviteOutcome::Inserted)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        InviteOutcome::NoProfile => Ok(profile_not_found(&headers)),
        InviteOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        InviteOutcome::NotCreator => Ok(forbidden(
            &headers,
            "Tylko organizator może zapraszać uczestników",
        )),
        InviteOutcome::Inserted => Ok(Json(SuccessResponse { success: true }).into_response()),
    }
}

pub(in crate::api) async fn event_uninvite_user(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path((id, profile_id)): Path<(String, String)>,
) -> Result<Response> {
    let (_session, user) = match crate::api::state::require_auth_db(&headers).await {
        Ok(pair) => pair,
        Err(response) => return Ok(*response),
    };
    let event_uuid = match crate::api::parse_uuid_response(&id, "event", &headers) {
        Ok(uuid) => uuid,
        Err(response) => return Ok(*response),
    };
    let Ok(target_profile) = Uuid::parse_str(&profile_id) else {
        return Ok(validation_error(
            &headers,
            "Nieprawidłowy identyfikator profilu",
        ));
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            let Some(profile) = load_profile_for_user(conn, user_id)
                .await
                .map_err(into_diesel)?
            else {
                return Ok::<UninviteOutcome, diesel::result::Error>(UninviteOutcome::NoProfile);
            };
            let Some(event) = load_event_by_id(conn, event_uuid)
                .await
                .map_err(into_diesel)?
            else {
                return Ok(UninviteOutcome::NotFound);
            };
            if event.creator_id != profile.id {
                return Ok(UninviteOutcome::NotCreator);
            }
            diesel::delete(
                event_attendees::table
                    .filter(event_attendees::event_id.eq(event_uuid))
                    .filter(event_attendees::profile_id.eq(target_profile))
                    .filter(event_attendees::status.eq(INVITED_STATUS)),
            )
            .execute(conn)
            .await?;
            Ok(UninviteOutcome::Removed)
        }
        .scope_boxed()
    })
    .await
    .map_err(crate::error::AppError::from)?;

    match outcome {
        UninviteOutcome::NoProfile => Ok(profile_not_found(&headers)),
        UninviteOutcome::NotFound => Ok(not_found_event(&headers, &id)),
        UninviteOutcome::NotCreator => Ok(forbidden(
            &headers,
            "Tylko organizator może odwoływać zaproszenia",
        )),
        UninviteOutcome::Removed => Ok(Json(SuccessResponse { success: true }).into_response()),
    }
}

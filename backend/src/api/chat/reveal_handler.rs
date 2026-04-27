//! Tap-to-reveal audit endpoint for flagged chat messages.
//!
//! When a viewer chooses to unblur a message that the moderation
//! engine flagged or blocked, the client posts here. We insert one
//! audit row per (message, viewer) so abuse of the reveal action
//! (e.g. someone repeatedly unhiding hate-speech) is greppable.
//! The PK makes repeat reveals idempotent — a 200 always means
//! "your reveal is recorded", regardless of whether it was the
//! first or fifth tap.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::api::state::SuccessResponse;
use crate::app::AppContext;
use crate::db;
use crate::db::schema::chat_message_reveals::dsl as r;

type Result<T> = crate::error::AppResult<T>;

enum RevealOutcome {
    NotFound,
    Inserted,
}

pub(in crate::api) async fn message_reveal(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response> {
    let (_session, user) = auth_or_respond!(headers);

    let message_id = match parse_uuid_response(&id, "message", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let viewer_user_id = user.id;

    let outcome = db::with_viewer_tx(viewer, move |conn| {
        async move {
            // RLS on chat_message_reveals enforces both that the
            // viewer is a member of the message's conversation and
            // that they're inserting for themselves. The FK on
            // `message_id` enforces existence.
            //
            // We deliberately do NOT pre-check existence: an existence
            // probe before the RLS-gated insert would let any caller
            // distinguish "message doesn't exist" (404 from probe)
            // from "exists but not in your conversation" (RLS reject)
            // — i.e. enumerate every message UUID in the system. The
            // insert itself is the source of truth: success means the
            // viewer can see this message and it exists; any failure
            // collapses to NOT_FOUND so the responses are
            // indistinguishable from the outside.
            let insert_result = diesel::insert_into(r::chat_message_reveals)
                .values((
                    r::message_id.eq(message_id),
                    r::viewer_user_id.eq(viewer_user_id),
                    r::revealed_at.eq(chrono::Utc::now()),
                ))
                .on_conflict((r::message_id, r::viewer_user_id))
                .do_nothing()
                .execute(conn)
                .await;

            match insert_result {
                Ok(_) => Ok::<_, diesel::result::Error>(RevealOutcome::Inserted),
                Err(err) => {
                    // FK violation (message gone) and RLS rejection
                    // (not your conversation, or somebody else's
                    // viewer_user_id) both surface here. Log so real
                    // outages aren't silently 404'd, then collapse.
                    tracing::debug!(
                        message_id = %message_id,
                        viewer_user_id,
                        error = %err,
                        "chat reveal rejected (treating as not-found for response uniformity)"
                    );
                    Ok(RevealOutcome::NotFound)
                }
            }
        }
        .scope_boxed()
    })
    .await?;

    match outcome {
        RevealOutcome::NotFound => Ok(error_response(
            axum::http::StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Message not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        )),
        RevealOutcome::Inserted => Ok(Json(SuccessResponse { success: true }).into_response()),
    }
}

//! Per-conversation mute toggle. POST = mute, DELETE = unmute.
//!
//! Membership check is the gate: muting a conversation you don't belong
//! to has no semantic meaning, so we 404 rather than silently insert a
//! dangling row. RLS on `conversation_mutes` (own-row-only) is the second
//! line of defence — a forged `user_id` would be rejected by the policy.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::common::{auth_or_respond, error_response, parse_uuid_response, ErrorSpec};
use crate::app::AppContext;
use crate::db;

type Result<T> = crate::error::AppResult<T>;

async fn require_member(
    conn: &mut diesel_async::AsyncPgConnection,
    conversation_id: Uuid,
    user_id: i32,
) -> std::result::Result<bool, diesel::result::Error> {
    use crate::db::schema::conversation_members;
    let count: i64 = conversation_members::table
        .filter(conversation_members::conversation_id.eq(conversation_id))
        .filter(conversation_members::user_id.eq(user_id))
        .count()
        .get_result(conn)
        .await?;
    Ok(count > 0)
}

pub async fn mute_conversation(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
) -> Result<Response> {
    use crate::db::schema::conversation_mutes;

    let (_session, user) = auth_or_respond!(headers);

    let conversation_id = match parse_uuid_response(&conversation_id, "conversation", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let caller_user_id = user.id;

    let outcome: bool = db::with_viewer_tx(viewer, move |conn| {
        async move {
            if !require_member(conn, conversation_id, caller_user_id).await? {
                return Ok::<bool, diesel::result::Error>(false);
            }
            diesel::insert_into(conversation_mutes::table)
                .values((
                    conversation_mutes::user_id.eq(caller_user_id),
                    conversation_mutes::conversation_id.eq(conversation_id),
                    conversation_mutes::muted_at.eq(chrono::Utc::now()),
                ))
                .on_conflict((
                    conversation_mutes::user_id,
                    conversation_mutes::conversation_id,
                ))
                .do_nothing()
                .execute(conn)
                .await?;
            Ok(true)
        }
        .scope_boxed()
    })
    .await?;

    if !outcome {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            &headers,
            ErrorSpec {
                error: "Conversation not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response())
}

pub async fn unmute_conversation(
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
    Path(conversation_id): Path<String>,
) -> Result<Response> {
    use crate::db::schema::conversation_mutes;

    let (_session, user) = auth_or_respond!(headers);

    let conversation_id = match parse_uuid_response(&conversation_id, "conversation", &headers) {
        Ok(id) => id,
        Err(response) => return Ok(*response),
    };

    let viewer = db::DbViewer {
        user_id: user.id,
        is_review_stub: user.is_review_stub,
    };
    let caller_user_id = user.id;

    db::with_viewer_tx(viewer, move |conn| {
        async move {
            diesel::delete(
                conversation_mutes::table
                    .filter(conversation_mutes::user_id.eq(caller_user_id))
                    .filter(conversation_mutes::conversation_id.eq(conversation_id)),
            )
            .execute(conn)
            .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response())
}

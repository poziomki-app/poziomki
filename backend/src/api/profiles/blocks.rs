use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::state::{DataResponse, SuccessResponse};
use crate::api::{error_response, ErrorSpec};
use crate::db::models::profile_blocks::ProfileBlock;
use crate::db::models::reports::NewReport;
use crate::db::schema::{profile_blocks, profiles, reports};

pub(in crate::api) async fn profile_block(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    my_profile_id: Uuid,
    target_id: Uuid,
) -> crate::error::AppResult<Response> {
    if my_profile_id == target_id {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Nie możesz zablokować siebie".to_string(),
                code: "VALIDATION_ERROR",
                details: None,
            },
        ));
    }

    // Verify target profile exists
    let target_exists = profiles::table
        .find(target_id)
        .select(profiles::id)
        .first::<Uuid>(conn)
        .await
        .optional()?;
    if target_exists.is_none() {
        return Ok(error_response(
            StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: "Profile not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ));
    }

    let now = Utc::now();
    let inserted = diesel::insert_into(profile_blocks::table)
        .values(&ProfileBlock {
            blocker_id: my_profile_id,
            blocked_id: target_id,
            created_at: now,
        })
        .on_conflict((profile_blocks::blocker_id, profile_blocks::blocked_id))
        .do_nothing()
        .execute(conn)
        .await?;

    // Apple guideline 1.2: blocking notifies the developer of the offending
    // user. Record a moderation report (deduped by the unique target index)
    // and fire a best-effort dev push. Only on a fresh block, to avoid spam.
    if inserted > 0 {
        let report = NewReport {
            reporter_id: my_profile_id,
            target_type: "profile".to_string(),
            target_id,
            reason: "block".to_string(),
            description: None,
        };
        if let Err(err) = diesel::insert_into(reports::table)
            .values(&report)
            .on_conflict_do_nothing()
            .execute(conn)
            .await
        {
            tracing::warn!(%target_id, error = %err, "failed to record block report");
        }
        tokio::spawn(notify_block_dev(my_profile_id, target_id));
    }

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

/// Best-effort developer notification when a user blocks another user.
/// Posts to an ntfy topic; failures are swallowed so blocking never breaks.
async fn notify_block_dev(blocker: Uuid, target: Uuid) {
    let url = std::env::var("NTFY_DEV_URL")
        .unwrap_or_else(|_| "https://ntfy.poziomki.app/poziomki-dev".to_string());
    let body = format!("block: {blocker} -> {target}");
    let result = reqwest::Client::new()
        .post(&url)
        .header("Title", "Poziomki: użytkownik zablokowany")
        .header("Tags", "no_entry")
        .body(body)
        .send()
        .await;
    if let Err(err) = result {
        tracing::warn!(error = %err, "failed to send block dev notification");
    }
}

pub(in crate::api) async fn profile_unblock(
    conn: &mut AsyncPgConnection,
    my_profile_id: Uuid,
    target_id: Uuid,
) -> crate::error::AppResult<Response> {
    diesel::delete(
        profile_blocks::table
            .filter(profile_blocks::blocker_id.eq(my_profile_id))
            .filter(profile_blocks::blocked_id.eq(target_id)),
    )
    .execute(conn)
    .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

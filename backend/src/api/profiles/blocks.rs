use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::api::state::{DataResponse, SuccessResponse};
use crate::api::{error_response, ErrorSpec};
use crate::db::models::profile_blocks::ProfileBlock;
use crate::db::schema::{profile_blocks, profiles};

pub(in crate::api) async fn profile_block(
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

    let mut conn = crate::db::conn().await?;

    // Verify target profile exists
    let target_exists = profiles::table
        .find(target_id)
        .select(profiles::id)
        .first::<Uuid>(&mut conn)
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
    diesel::insert_into(profile_blocks::table)
        .values(&ProfileBlock {
            blocker_id: my_profile_id,
            blocked_id: target_id,
            created_at: now,
        })
        .on_conflict((profile_blocks::blocker_id, profile_blocks::blocked_id))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn profile_unblock(
    my_profile_id: Uuid,
    target_id: Uuid,
) -> crate::error::AppResult<Response> {
    let mut conn = crate::db::conn().await?;

    diesel::delete(
        profile_blocks::table
            .filter(profile_blocks::blocker_id.eq(my_profile_id))
            .filter(profile_blocks::blocked_id.eq(target_id)),
    )
    .execute(&mut conn)
    .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

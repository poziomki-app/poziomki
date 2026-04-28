use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::api::state::{DataResponse, SuccessResponse};
use crate::api::{error_response, ErrorSpec};
use crate::db;
use crate::db::models::profile_bookmarks::ProfileBookmark;
use crate::db::models::profiles::Profile;
use crate::db::schema::{profile_blocks, profile_bookmarks, profiles};

use super::profiles_view::profile_to_response;
use crate::api::state::ProfileResponse;

pub(in crate::api) async fn profile_bookmark(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    my_profile: &Profile,
    target_id: Uuid,
) -> crate::error::AppResult<Response> {
    if my_profile.id == target_id {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            headers,
            ErrorSpec {
                error: "Cannot bookmark yourself".to_string(),
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
    diesel::insert_into(profile_bookmarks::table)
        .values(&ProfileBookmark {
            profile_id: my_profile.id,
            target_profile_id: target_id,
            created_at: now,
        })
        .on_conflict((
            profile_bookmarks::profile_id,
            profile_bookmarks::target_profile_id,
        ))
        .do_nothing()
        .execute(conn)
        .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn profile_unbookmark(
    conn: &mut AsyncPgConnection,
    my_profile_id: Uuid,
    target_id: Uuid,
) -> crate::error::AppResult<Response> {
    diesel::delete(
        profile_bookmarks::table
            .filter(profile_bookmarks::profile_id.eq(my_profile_id))
            .filter(profile_bookmarks::target_profile_id.eq(target_id)),
    )
    .execute(conn)
    .await?;

    Ok(Json(DataResponse {
        data: SuccessResponse { success: true },
    })
    .into_response())
}

pub(in crate::api) async fn profiles_bookmarked(
    conn: &mut AsyncPgConnection,
    my_profile_id: Uuid,
    viewer_user_id: i32,
) -> crate::error::AppResult<Vec<ProfileResponse>> {
    let bookmarked_profiles: Vec<(ProfileBookmark, Profile)> = profile_bookmarks::table
        .inner_join(profiles::table.on(profile_bookmarks::target_profile_id.eq(profiles::id)))
        .filter(profile_bookmarks::profile_id.eq(my_profile_id))
        .order(profile_bookmarks::created_at.desc())
        .select((profile_bookmarks::all_columns, profiles::all_columns))
        .load(conn)
        .await?;

    // Filter symmetric blocks. Mirrors matching/repo.rs:209-234 and
    // search.rs — without this, a profile that bookmarked someone
    // before either side blocked still sees the blocked profile's
    // full data on `GET /api/v1/profiles/bookmarked`.
    let block_rows: Vec<(Uuid, Uuid)> = profile_blocks::table
        .filter(
            profile_blocks::blocker_id
                .eq(my_profile_id)
                .or(profile_blocks::blocked_id.eq(my_profile_id)),
        )
        .select((profile_blocks::blocker_id, profile_blocks::blocked_id))
        .load(conn)
        .await?;
    let blocked: std::collections::HashSet<Uuid> = block_rows
        .into_iter()
        .map(|(a, b)| if a == my_profile_id { b } else { a })
        .collect();

    let mut responses = Vec::with_capacity(bookmarked_profiles.len());
    for (_bookmark, profile) in bookmarked_profiles {
        if blocked.contains(&profile.id) {
            continue;
        }
        // Narrow public-projection helper avoids reading the owner's full
        // users row just to get their pid.
        let owner_pid = db::user_pid_for_id(conn, profile.user_id)
            .await?
            .unwrap_or_else(Uuid::nil);
        let response = profile_to_response(conn, &profile, &owner_pid, Some(viewer_user_id)).await;
        responses.push(response);
    }
    Ok(responses)
}

pub(in crate::api) async fn is_bookmarked(
    conn: &mut AsyncPgConnection,
    my_profile_id: Uuid,
    target_profile_id: Uuid,
) -> crate::error::AppResult<bool> {
    let exists = profile_bookmarks::table
        .find((my_profile_id, target_profile_id))
        .select(profile_bookmarks::profile_id)
        .first::<Uuid>(conn)
        .await
        .optional()?;
    Ok(exists.is_some())
}

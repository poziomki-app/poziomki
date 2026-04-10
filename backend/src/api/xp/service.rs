use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::schema::profiles;

/// Award XP to a profile and update streak in a single UPDATE.
/// Streak logic:
///   - `streak_last_active` == today  → streak unchanged
///   - `streak_last_active` == yesterday → streak incremented
///   - anything else (gap or null)    → streak reset to 1
pub async fn award_xp(profile_id: Uuid, amount: i32) -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let today = Utc::now().date_naive();

    diesel::sql_query(
        "UPDATE profiles
         SET xp = xp + $1,
             streak_current = CASE
                 WHEN streak_last_active = $2 THEN streak_current
                 WHEN streak_last_active = $2 - INTERVAL '1 day' THEN streak_current + 1
                 ELSE 1
             END,
             streak_longest = GREATEST(streak_longest, CASE
                 WHEN streak_last_active = $2 THEN streak_current
                 WHEN streak_last_active = $2 - INTERVAL '1 day' THEN streak_current + 1
                 ELSE 1
             END),
             streak_last_active = $2
         WHERE id = $3",
    )
    .bind::<diesel::sql_types::Integer, _>(amount)
    .bind::<diesel::sql_types::Date, _>(today)
    .bind::<diesel::sql_types::Uuid, _>(profile_id)
    .execute(&mut conn)
    .await?;

    Ok(())
}

/// Check whether a task was already completed today by this profile.
/// Inserts if not; returns true if XP should be awarded.
pub async fn try_record_task(
    profile_id: Uuid,
    task_id: &str,
) -> Result<bool, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;

    let inserted = diesel::sql_query(
        "INSERT INTO task_completions (profile_id, task_id, day) \
         VALUES ($1, $2, CURRENT_DATE) \
         ON CONFLICT DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(profile_id)
    .bind::<diesel::sql_types::Text, _>(task_id)
    .execute(&mut conn)
    .await?;

    Ok(inserted > 0)
}

/// Check whether a scan pair (scanner, scanned) already occurred today.
/// Inserts the record if not; returns true if XP should be awarded.
pub async fn try_record_scan(from_id: Uuid, to_id: Uuid) -> Result<bool, crate::error::AppError> {
    use crate::db::schema::xp_scans;
    let mut conn = crate::db::conn().await?;
    let today = Utc::now().date_naive();

    // Try to insert; if duplicate primary key, returns 0 rows affected.
    let inserted = diesel::insert_into(xp_scans::table)
        .values((
            xp_scans::scanner_id.eq(from_id),
            xp_scans::scanned_id.eq(to_id),
            xp_scans::day.eq(today),
        ))
        .on_conflict((xp_scans::scanner_id, xp_scans::scanned_id, xp_scans::day))
        .do_nothing()
        .execute(&mut conn)
        .await?;

    Ok(inserted > 0)
}

/// Resolve the profile ID for the current user's API key / session from headers.
pub async fn load_profile_for(
    headers: &axum::http::HeaderMap,
) -> Result<crate::db::models::profiles::Profile, Box<axum::response::Response>> {
    use crate::api::{error_response, ErrorSpec};

    let (_session, user) = crate::api::state::require_auth_db(headers).await?;

    let mut conn = crate::db::conn().await.map_err(|_| {
        Box::new(error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Database error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ))
    })?;

    profiles::table
        .filter(profiles::user_id.eq(user.id))
        .first::<crate::db::models::profiles::Profile>(&mut conn)
        .await
        .optional()
        .map_err(|_| {
            Box::new(error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: "Profile not found".to_string(),
                    code: "NOT_FOUND",
                    details: None,
                },
            ))
        })?
        .ok_or_else(|| {
            Box::new(error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: "Profile not found".to_string(),
                    code: "NOT_FOUND",
                    details: None,
                },
            ))
        })
}

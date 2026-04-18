use axum::http::HeaderMap;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db;
use crate::db::schema::profiles;

/// Award XP + bump streak for a profile. Delegates to the
/// `app.award_profile_xp` `SECURITY DEFINER` helper so the award works from
/// spawned background tasks (no viewer context) and from cross-profile
/// credits (scanner awarding the scanned profile). Streak rules live in
/// the migration, not inline SQL, so they stay in one place.
pub async fn award_xp(profile_id: Uuid, amount: i32) -> Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    db::award_profile_xp(&mut conn, profile_id, amount).await?;
    Ok(())
}

/// Record that `profile_id` completed `task_id` today. Idempotent per
/// `(profile_id, task_id, day)`; returns `true` iff XP should be awarded.
/// Caller supplies a connection already inside a viewer-scoped tx.
pub async fn try_record_task(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
    task_id: &str,
) -> Result<bool, crate::error::AppError> {
    let inserted = diesel::sql_query(
        "INSERT INTO task_completions (profile_id, task_id, day) \
         VALUES ($1, $2, CURRENT_DATE) \
         ON CONFLICT DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(profile_id)
    .bind::<diesel::sql_types::Text, _>(task_id)
    .execute(conn)
    .await?;

    Ok(inserted > 0)
}

/// Record that `from_id` scanned `to_id` today. Idempotent per
/// `(scanner_id, scanned_id, day)`; returns `true` iff XP should be
/// awarded. Caller supplies a connection already inside a viewer-scoped
/// tx keyed to `from_id`'s owner.
pub async fn try_record_scan(
    conn: &mut AsyncPgConnection,
    from_id: Uuid,
    to_id: Uuid,
) -> Result<bool, crate::error::AppError> {
    use crate::db::schema::xp_scans;
    let today = chrono::Utc::now().date_naive();

    let inserted = diesel::insert_into(xp_scans::table)
        .values((
            xp_scans::scanner_id.eq(from_id),
            xp_scans::scanned_id.eq(to_id),
            xp_scans::day.eq(today),
        ))
        .on_conflict((xp_scans::scanner_id, xp_scans::scanned_id, xp_scans::day))
        .do_nothing()
        .execute(conn)
        .await?;

    Ok(inserted > 0)
}

/// Load the caller's profile inside an existing viewer-scoped transaction.
/// Returns a 404 response shell on missing profile; Diesel-level failures
/// surface as 500 so real DB issues don't get masked as "profile not found".
pub async fn load_profile_for_user(
    conn: &mut AsyncPgConnection,
    headers: &HeaderMap,
    user_id: i32,
) -> Result<crate::db::models::profiles::Profile, Box<axum::response::Response>> {
    use crate::api::{error_response, ErrorSpec};

    match profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<crate::db::models::profiles::Profile>(conn)
        .await
        .optional()
    {
        Ok(Some(profile)) => Ok(profile),
        Ok(None) => Err(Box::new(error_response(
            axum::http::StatusCode::NOT_FOUND,
            headers,
            ErrorSpec {
                error: "Profile not found".to_string(),
                code: "NOT_FOUND",
                details: None,
            },
        ))),
        Err(_) => Err(Box::new(error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers,
            ErrorSpec {
                error: "Database error".to_string(),
                code: "INTERNAL_ERROR",
                details: None,
            },
        ))),
    }
}

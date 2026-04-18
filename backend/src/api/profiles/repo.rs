use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db;
use crate::db::models::profiles::Profile;
use crate::db::schema::profiles;

pub(super) async fn load_profile_by_user_id(
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> std::result::Result<Option<Profile>, crate::error::AppError> {
    profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(conn)
        .await
        .optional()
        .map_err(Into::into)
}

pub(super) async fn load_profile_with_owner_pid(
    conn: &mut AsyncPgConnection,
    profile_id: Uuid,
) -> std::result::Result<Option<(Profile, Uuid)>, crate::error::AppError> {
    let profile = profiles::table
        .find(profile_id)
        .first::<Profile>(conn)
        .await
        .optional()?;

    let Some(profile) = profile else {
        return Ok(None);
    };

    // Narrow public-projection helper: only returns the owner's pid, never
    // the full users row (which carries password hash, email, etc.).
    let owner_pid = db::user_pid_for_id(conn, profile.user_id)
        .await?
        .unwrap_or_else(Uuid::nil);

    Ok(Some((profile, owner_pid)))
}

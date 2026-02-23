use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::profiles::Profile;
use crate::db::models::users::User;
use crate::db::schema::{profiles, users};

pub(super) async fn load_profile_by_user_id(
    user_id: i32,
) -> std::result::Result<Option<Profile>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(&mut conn)
        .await
        .optional()
        .map_err(Into::into)
}

pub(super) async fn load_profile_with_owner_pid(
    profile_id: Uuid,
) -> std::result::Result<Option<(Profile, Uuid)>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let profile = profiles::table
        .find(profile_id)
        .first::<Profile>(&mut conn)
        .await
        .optional()?;

    let Some(profile) = profile else {
        return Ok(None);
    };

    let owner = users::table
        .find(profile.user_id)
        .first::<User>(&mut conn)
        .await
        .optional()?;
    let user_pid = owner.map_or(Uuid::nil(), |u| u.pid);

    Ok(Some((profile, user_pid)))
}

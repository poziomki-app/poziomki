use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::profiles::Profile;
use crate::db::models::uploads::Upload;
use crate::db::schema::{profiles, uploads};

pub(in crate::api) async fn load_profile_by_user_id(
    user_id: i32,
) -> std::result::Result<Option<Profile>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let profile = profiles::table
        .filter(profiles::user_id.eq(user_id))
        .first::<Profile>(&mut conn)
        .await
        .optional()?;
    Ok(profile)
}

pub(in crate::api) async fn find_active_upload_by_filename(
    filename: &str,
) -> std::result::Result<Option<Upload>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let upload = uploads::table
        .filter(uploads::filename.eq(filename))
        .filter(uploads::deleted.eq(false))
        .first::<Upload>(&mut conn)
        .await
        .optional()?;
    Ok(upload)
}

pub(in crate::api) async fn find_owned_active_upload_by_filenames(
    owner_profile_id: Uuid,
    filenames: &[String],
) -> std::result::Result<Option<Upload>, crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    let upload = uploads::table
        .filter(uploads::owner_id.eq(Some(owner_profile_id)))
        .filter(uploads::deleted.eq(false))
        .filter(uploads::filename.eq_any(filenames))
        .first::<Upload>(&mut conn)
        .await
        .optional()?;
    Ok(upload)
}

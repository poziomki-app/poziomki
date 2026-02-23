use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::db::models::uploads::{NewUpload, UploadChangeset};
use crate::db::schema::uploads;

pub(in crate::api) async fn insert_upload_metadata(
    new_upload: &NewUpload,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::insert_into(uploads::table)
        .values(new_upload)
        .execute(&mut conn)
        .await?;
    Ok(())
}

pub(in crate::api) async fn mark_upload_deleted(
    upload_id: Uuid,
    changeset: &UploadChangeset,
) -> std::result::Result<(), crate::error::AppError> {
    let mut conn = crate::db::conn().await?;
    diesel::update(uploads::table.find(upload_id))
        .set(changeset)
        .execute(&mut conn)
        .await?;
    Ok(())
}

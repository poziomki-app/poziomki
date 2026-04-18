use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::db::models::uploads::{NewUpload, UploadChangeset};
use crate::db::schema::uploads;

pub(in crate::api) async fn insert_upload_metadata(
    conn: &mut AsyncPgConnection,
    new_upload: &NewUpload,
) -> std::result::Result<(), crate::error::AppError> {
    diesel::insert_into(uploads::table)
        .values(new_upload)
        .execute(conn)
        .await?;
    Ok(())
}

pub(in crate::api) async fn mark_upload_deleted(
    conn: &mut AsyncPgConnection,
    upload_id: Uuid,
    changeset: &UploadChangeset,
) -> std::result::Result<(), crate::error::AppError> {
    diesel::update(uploads::table.find(upload_id))
        .set(changeset)
        .execute(conn)
        .await?;
    Ok(())
}

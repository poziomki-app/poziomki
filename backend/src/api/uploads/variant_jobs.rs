use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::{uploads_resize, uploads_storage};
use crate::db::models::uploads::UploadChangeset;
use crate::db::schema::uploads;

pub(super) async fn generate_upload_variants_job(
    upload_id: Uuid,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|error| error.to_string())?;

    let Some(upload) = uploads::table
        .find(upload_id)
        .first::<crate::db::models::uploads::Upload>(&mut conn)
        .await
        .optional()
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };

    if upload.deleted || upload.has_variants {
        return Ok(());
    }

    let original_bytes = uploads_storage::read(&upload.filename)
        .await
        .map_err(|error| format!("read original upload failed: {error:?}"))?;

    let thumbhash = uploads_resize::compute_thumbhash(&original_bytes, &upload.mime_type).await?;

    let avif_mime = match uploads_resize::encode_avif(&original_bytes, &upload.mime_type).await {
        Ok(avif_bytes) => {
            if let Err(err) =
                uploads_storage::upload(&upload.filename, &avif_bytes, "image/avif").await
            {
                tracing::warn!(filename = %upload.filename, ?err, "failed to overwrite original with AVIF");
                None
            } else {
                Some("image/avif".to_string())
            }
        }
        Err(reason) => {
            if upload.mime_type != "image/avif" {
                tracing::warn!(filename = %upload.filename, %reason, "AVIF re-encode skipped");
            }
            None
        }
    };

    let changeset = UploadChangeset {
        thumbhash: Some(Some(thumbhash)),
        mime_type: avif_mime,
        has_variants: Some(true),
        updated_at: Some(Utc::now()),
        ..Default::default()
    };

    diesel::update(uploads::table.find(upload.id))
        .set(&changeset)
        .execute(&mut conn)
        .await
        .map_err(|error| format!("update upload thumbhash metadata failed: {error}"))?;

    Ok(())
}

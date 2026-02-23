use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::{uploads_resize, uploads_storage};
use crate::db::models::uploads::{Upload, UploadChangeset};
use crate::db::schema::uploads;

struct VariantUploadResult {
    thumb: std::result::Result<(), uploads_storage::StorageError>,
    std: std::result::Result<(), uploads_storage::StorageError>,
}

/// Checks both variant uploads succeeded; on partial failure, cleans up and returns an error.
async fn ensure_both_variants_uploaded(
    results: VariantUploadResult,
    thumb_name: &str,
    std_name: &str,
) -> std::result::Result<(), String> {
    match (results.thumb, results.std) {
        (Ok(()), Ok(())) => Ok(()),
        (thumb, std) => {
            if thumb.is_ok() {
                let _ = uploads_storage::delete(thumb_name).await;
            }
            if std.is_ok() {
                let _ = uploads_storage::delete(std_name).await;
            }
            let thumb_err = thumb.err().map(|e| format!("{e:?}"));
            let std_err = std.err().map(|e| format!("{e:?}"));
            Err(format!(
                "variant upload incomplete: thumb_err={thumb_err:?} std_err={std_err:?}"
            ))
        }
    }
}

pub(super) async fn generate_upload_variants_job(
    upload_id: Uuid,
) -> std::result::Result<(), String> {
    let mut conn = crate::db::conn().await.map_err(|error| error.to_string())?;

    let Some(upload) = uploads::table
        .find(upload_id)
        .first::<Upload>(&mut conn)
        .await
        .optional()
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };

    if upload.deleted || upload.has_variants {
        return Ok(());
    }

    process_upload_variants(&upload, &mut conn).await
}

async fn process_upload_variants(
    upload: &Upload,
    conn: &mut crate::db::DbConn,
) -> std::result::Result<(), String> {
    let original_bytes = uploads_storage::read(&upload.filename)
        .await
        .map_err(|error| format!("read original upload failed: {error:?}"))?;

    let variants = uploads_resize::generate_variants(&original_bytes, &upload.mime_type).await?;
    let thumb_name = uploads_resize::variant_filename(&upload.filename, "thumb");
    let std_name = uploads_resize::variant_filename(&upload.filename, "std");

    let (thumb_upload, std_upload) = tokio::join!(
        uploads_storage::upload(&thumb_name, &variants.thumbnail, "image/webp"),
        uploads_storage::upload(&std_name, &variants.standard, "image/webp")
    );

    ensure_both_variants_uploaded(
        VariantUploadResult {
            thumb: thumb_upload,
            std: std_upload,
        },
        &thumb_name,
        &std_name,
    )
    .await?;

    let changeset = UploadChangeset {
        thumbhash: Some(Some(variants.thumbhash)),
        has_variants: Some(true),
        updated_at: Some(Utc::now()),
        ..Default::default()
    };

    diesel::update(uploads::table.find(upload.id))
        .set(&changeset)
        .execute(conn)
        .await
        .map_err(|error| format!("update upload variants metadata failed: {error}"))?;

    Ok(())
}

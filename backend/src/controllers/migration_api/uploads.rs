#[path = "uploads_multipart.rs"]
mod uploads_multipart;
#[path = "uploads_storage.rs"]
mod uploads_storage;
#[path = "uploads_support.rs"]
mod uploads_support;

use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, HeaderValue},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveValue, QueryFilter};
use uuid::Uuid;

use super::state::{
    create_upload_filename, is_production_mode, require_auth_db, validate_filename, DataResponse,
    SuccessResponse, UploadResponse, UploadUrlResponse,
};
use crate::models::_entities::{profiles, uploads};
use uploads_multipart::HandlerError;
use uploads_support::{
    bad_request, not_found, storage_delete, storage_exists, storage_read, storage_signed_url,
    storage_upload,
};

pub(super) struct AuthCheckResponse {
    pub(super) ok: bool,
}

impl serde::Serialize for AuthCheckResponse {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AuthCheckResponse", 1)?;
        state.serialize_field("ok", &self.ok)?;
        state.end()
    }
}

async fn production_file_get(
    db: &DatabaseConnection,
    headers: HeaderMap,
    filename: String,
) -> Response {
    let response: std::result::Result<Response, HandlerError> = async {
        require_auth_db(db, &headers)
            .await
            .map_err(|e| e as HandlerError)?;

        if !storage_exists(&headers, &filename).await? {
            return Err(Box::new(not_found(&headers)));
        }

        let url = storage_signed_url(&headers, &filename).await?;
        Ok(Json(UploadUrlResponse { url }).into_response())
    }
    .await;

    response.unwrap_or_else(|response| *response)
}

async fn development_file_get(
    db: &DatabaseConnection,
    filename: String,
    headers: HeaderMap,
) -> Response {
    let mime_type = uploads::Entity::find()
        .filter(uploads::Column::Filename.eq(&filename))
        .filter(uploads::Column::Deleted.eq(false))
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|record| record.mime_type);

    let Some(mime_type) = mime_type else {
        return not_found(&headers);
    };

    let bytes = match storage_read(&headers, &filename).await {
        Ok(value) => value,
        Err(response) => return *response,
    };

    let mut response = bytes.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000"),
    );

    response
}

pub(super) async fn auth_check(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let (_session, user) = require_auth_db(&ctx.db, &headers)
            .await
            .map_err(|e| e as HandlerError)?;

        let _profile = profiles::Entity::find()
            .filter(profiles::Column::UserId.eq(user.id))
            .one(&ctx.db)
            .await
            .map_err(|_| {
                Box::new(uploads_support::forbidden(
                    &headers,
                    "ACCESS_DENIED",
                    "Profile not found",
                )) as HandlerError
            })?
            .ok_or_else(|| {
                Box::new(uploads_support::forbidden(
                    &headers,
                    "ACCESS_DENIED",
                    "Profile not found",
                )) as HandlerError
            })?;

        Ok(Json(AuthCheckResponse { ok: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|r| *r))
}

pub(super) async fn file_get(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    if let Err(message) = validate_filename(&filename) {
        return Ok(bad_request(&headers, "INVALID_FILENAME", message));
    }

    if is_production_mode() {
        Ok(production_file_get(&ctx.db, headers, filename).await)
    } else {
        Ok(development_file_get(&ctx.db, filename, headers).await)
    }
}

pub(super) async fn file_upload(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        let owner_id = {
            let (_session, user) = require_auth_db(&ctx.db, &headers)
                .await
                .map_err(|e| e as HandlerError)?;
            profiles::Entity::find()
                .filter(profiles::Column::UserId.eq(user.id))
                .one(&ctx.db)
                .await
                .ok()
                .flatten()
                .map(|p| p.id)
        };

        let parsed = uploads_multipart::read_multipart(&headers, multipart).await?;
        let filename = create_upload_filename(&parsed.mime_type);
        storage_upload(&headers, &filename, &parsed.bytes, &parsed.mime_type).await?;

        // Store upload record in DB
        let context_str = format!("{:?}", parsed.context).to_ascii_lowercase();
        let now = Utc::now();
        let upload = uploads::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            filename: ActiveValue::Set(filename.clone()),
            owner_id: ActiveValue::Set(owner_id),
            context: ActiveValue::Set(context_str),
            context_id: ActiveValue::Set(parsed.context_id),
            mime_type: ActiveValue::Set(parsed.mime_type.clone()),
            deleted: ActiveValue::Set(false),
            created_at: ActiveValue::Set(now.into()),
            updated_at: ActiveValue::Set(now.into()),
        };
        let _ = upload.insert(&ctx.db).await;

        let url = if is_production_mode() {
            storage_signed_url(&headers, &filename).await?
        } else {
            format!("/api/v1/uploads/{filename}")
        };

        Ok(Json(DataResponse {
            data: UploadResponse {
                url,
                filename,
                size: parsed.bytes.len(),
                mime_type: parsed.mime_type,
            },
        })
        .into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|resp| *resp))
}

pub(super) async fn file_delete(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> Result<Response> {
    let response: std::result::Result<Response, HandlerError> = async {
        if let Err(message) = validate_filename(&filename) {
            return Err(Box::new(bad_request(&headers, "INVALID_FILENAME", message)));
        }

        require_auth_db(&ctx.db, &headers)
            .await
            .map_err(|e| e as HandlerError)?;

        let upload = uploads::Entity::find()
            .filter(uploads::Column::Filename.eq(&filename))
            .filter(uploads::Column::Deleted.eq(false))
            .one(&ctx.db)
            .await
            .map_err(|_| Box::new(not_found(&headers)) as HandlerError)?
            .ok_or_else(|| Box::new(not_found(&headers)) as HandlerError)?;

        storage_delete(&headers, &filename).await?;

        // Mark as deleted
        let mut active: uploads::ActiveModel = upload.into();
        active.deleted = ActiveValue::Set(true);
        active.updated_at = ActiveValue::Set(Utc::now().into());
        let _ = active.update(&ctx.db).await;

        Ok(Json(SuccessResponse { success: true }).into_response())
    }
    .await;

    Ok(response.unwrap_or_else(|response| *response))
}

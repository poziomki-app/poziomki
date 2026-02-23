use axum::response::Response;
use axum::{
    extract::{multipart::Field, Multipart},
    http::{HeaderMap, StatusCode},
};

use crate::controllers::api::{
    error_response,
    state::{
        allowed_upload_mime, is_chat_context, max_upload_size_bytes, parse_upload_context,
        validate_image_dimensions, validate_magic_bytes, UploadContext,
    },
    ErrorSpec,
};

pub(in crate::controllers::api) type HandlerError = Box<Response>;
pub(in crate::controllers::api) type HandlerResult<T> = std::result::Result<T, HandlerError>;

pub(in crate::controllers::api) struct ParsedUpload {
    pub(in crate::controllers::api) context: UploadContext,
    pub(in crate::controllers::api) context_id: Option<String>,
    pub(in crate::controllers::api) mime_type: String,
    pub(in crate::controllers::api) bytes: Vec<u8>,
}

#[derive(Default)]
struct UploadDraft {
    context: Option<UploadContext>,
    context_id: Option<String>,
    file_mime: Option<String>,
    file_bytes: Option<Vec<u8>>,
}

enum UploadFieldKind {
    Context,
    ContextId,
    File,
    Ignore,
}

fn bad_request(headers: &HeaderMap, code: &'static str, message: &str) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        headers,
        ErrorSpec {
            error: message.to_string(),
            code,
            details: None,
        },
    )
}

fn parse_context_field(headers: &HeaderMap, value: &str) -> HandlerResult<UploadContext> {
    parse_upload_context(Some(value)).ok_or_else(|| {
        Box::new(bad_request(
            headers,
            "VALIDATION_ERROR",
            "Invalid upload context",
        ))
    })
}

fn parse_upload_mime(headers: &HeaderMap, mime_type: &str) -> HandlerResult<()> {
    if !allowed_upload_mime(mime_type) {
        return Err(Box::new(bad_request(
            headers,
            "INVALID_FILE_TYPE",
            "Allowed: image/jpeg, image/png, image/webp",
        )));
    }

    Ok(())
}

fn validate_chat_context(headers: &HeaderMap, parsed: &ParsedUpload) -> HandlerResult<()> {
    if is_chat_context(parsed.context) && parsed.context_id.is_none() {
        return Err(Box::new(bad_request(
            headers,
            "MISSING_CONTEXT_ID",
            "contextId required for chat uploads",
        )));
    }
    Ok(())
}

fn validate_upload_size(headers: &HeaderMap, parsed: &ParsedUpload) -> HandlerResult<()> {
    if parsed.bytes.len() > max_upload_size_bytes() {
        return Err(Box::new(bad_request(
            headers,
            "FILE_TOO_LARGE",
            "Max: 10MB",
        )));
    }
    Ok(())
}

fn validate_upload_content(headers: &HeaderMap, parsed: &ParsedUpload) -> HandlerResult<()> {
    if !validate_magic_bytes(&parsed.bytes, &parsed.mime_type) {
        return Err(Box::new(bad_request(
            headers,
            "INVALID_FILE_CONTENT",
            "Content does not match type",
        )));
    }
    Ok(())
}

fn validate_upload_dimensions(headers: &HeaderMap, parsed: &ParsedUpload) -> HandlerResult<()> {
    validate_image_dimensions(&parsed.bytes, &parsed.mime_type)
        .map_err(|msg| Box::new(bad_request(headers, "IMAGE_TOO_LARGE", msg)))
}

fn validate_upload_payload(headers: &HeaderMap, parsed: &ParsedUpload) -> HandlerResult<()> {
    validate_chat_context(headers, parsed)?;
    parse_upload_mime(headers, &parsed.mime_type)?;
    validate_upload_size(headers, parsed)?;
    validate_upload_content(headers, parsed)?;
    validate_upload_dimensions(headers, parsed)
}

fn classify_field(name: Option<&str>) -> UploadFieldKind {
    match name {
        Some("context") => UploadFieldKind::Context,
        Some("contextId") => UploadFieldKind::ContextId,
        Some("file") => UploadFieldKind::File,
        _ => UploadFieldKind::Ignore,
    }
}

async fn parse_text_field(
    headers: &HeaderMap,
    field: Field<'_>,
    field_name: &'static str,
) -> HandlerResult<String> {
    field
        .text()
        .await
        .map_err(|_| Box::new(bad_request(headers, "VALIDATION_ERROR", field_name)))
}

async fn read_file_field(
    headers: &HeaderMap,
    mut field: Field<'_>,
) -> HandlerResult<(String, Vec<u8>)> {
    let mime_type = field
        .content_type()
        .map(ToOwned::to_owned)
        .unwrap_or_default();

    let max_size = max_upload_size_bytes();
    let mut total_size = 0usize;
    let mut bytes = Vec::new();

    while let Some(chunk) = field.chunk().await.map_err(|_| {
        Box::new(bad_request(
            headers,
            "VALIDATION_ERROR",
            "Invalid file field",
        ))
    })? {
        let chunk_size = chunk.len();
        total_size = total_size.saturating_add(chunk_size);
        if total_size > max_size {
            return Err(Box::new(bad_request(
                headers,
                "FILE_TOO_LARGE",
                "Max: 10MB",
            )));
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok((mime_type, bytes))
}

async fn apply_context_field(
    headers: &HeaderMap,
    draft: &mut UploadDraft,
    field: Field<'_>,
) -> HandlerResult<()> {
    let value = parse_text_field(headers, field, "Invalid context field").await?;
    draft.context = Some(parse_context_field(headers, &value)?);
    Ok(())
}

async fn apply_context_id_field(
    headers: &HeaderMap,
    draft: &mut UploadDraft,
    field: Field<'_>,
) -> HandlerResult<()> {
    let value = parse_text_field(headers, field, "Invalid contextId field").await?;
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        draft.context_id = Some(trimmed.to_string());
    }
    Ok(())
}

async fn apply_file_field(
    headers: &HeaderMap,
    draft: &mut UploadDraft,
    field: Field<'_>,
) -> HandlerResult<()> {
    let (mime_type, bytes) = read_file_field(headers, field).await?;
    draft.file_mime = Some(mime_type);
    draft.file_bytes = Some(bytes);
    Ok(())
}

async fn apply_field(
    headers: &HeaderMap,
    draft: &mut UploadDraft,
    field: Field<'_>,
) -> HandlerResult<()> {
    match classify_field(field.name()) {
        UploadFieldKind::Context => apply_context_field(headers, draft, field).await,
        UploadFieldKind::ContextId => apply_context_id_field(headers, draft, field).await,
        UploadFieldKind::File => apply_file_field(headers, draft, field).await,
        UploadFieldKind::Ignore => Ok(()),
    }
}

fn build_parsed_upload(headers: &HeaderMap, draft: UploadDraft) -> HandlerResult<ParsedUpload> {
    let UploadDraft {
        context,
        context_id,
        file_mime,
        file_bytes,
    } = draft;

    let bytes = file_bytes.ok_or_else(|| {
        Box::new(bad_request(
            headers,
            "VALIDATION_ERROR",
            "file field is required",
        ))
    })?;
    let mime_type = file_mime.ok_or_else(|| {
        Box::new(bad_request(
            headers,
            "INVALID_FILE_TYPE",
            "Allowed: image/jpeg, image/png, image/webp",
        ))
    })?;

    let parsed = ParsedUpload {
        context: context.unwrap_or(UploadContext::ProfileGallery),
        context_id,
        mime_type,
        bytes,
    };
    validate_upload_payload(headers, &parsed)?;
    Ok(parsed)
}

pub(in crate::controllers::api) async fn read_multipart(
    headers: &HeaderMap,
    mut multipart: Multipart,
) -> HandlerResult<ParsedUpload> {
    let mut draft = UploadDraft::default();

    while let Some(field) = multipart.next_field().await.map_err(|_| {
        Box::new(bad_request(
            headers,
            "VALIDATION_ERROR",
            "Invalid multipart payload",
        ))
    })? {
        apply_field(headers, &mut draft, field).await?;
    }

    build_parsed_upload(headers, draft)
}

use uuid::Uuid;

use super::state_types::UploadContext;

const MAX_UPLOAD_SIZE_BYTES: usize = 10 * 1024 * 1024;

pub(in crate::controllers::migration_api) fn is_production_mode() -> bool {
    std::env::var("NODE_ENV")
        .map(|value| value.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

pub(in crate::controllers::migration_api) const fn max_upload_size_bytes() -> usize {
    MAX_UPLOAD_SIZE_BYTES
}

pub(in crate::controllers::migration_api) fn allowed_upload_mime(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/jpeg" | "image/png" | "image/webp" | "image/avif"
    )
}

fn bytes_match_at(data: &[u8], offset: usize, expected: &[u8]) -> bool {
    let end = offset.checked_add(expected.len());
    end.and_then(|end_idx| data.get(offset..end_idx))
        .is_some_and(|slice| slice == expected)
}

pub(in crate::controllers::migration_api) fn validate_magic_bytes(
    data: &[u8],
    mime_type: &str,
) -> bool {
    match mime_type {
        "image/jpeg" => bytes_match_at(data, 0, &[0xff, 0xd8, 0xff]),
        "image/png" => bytes_match_at(data, 0, &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
        "image/webp" => {
            bytes_match_at(data, 0, &[0x52, 0x49, 0x46, 0x46])
                && bytes_match_at(data, 8, &[0x57, 0x45, 0x42, 0x50])
        }
        "image/avif" => {
            bytes_match_at(data, 4, &[0x66, 0x74, 0x79, 0x70])
                && bytes_match_at(data, 8, &[0x61, 0x76, 0x69, 0x66])
        }
        _ => false,
    }
}

pub(in crate::controllers::migration_api) fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" => "jpeg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/avif" => "avif",
        _ => "bin",
    }
}

pub(in crate::controllers::migration_api) fn create_upload_filename(mime_type: &str) -> String {
    let ext = extension_for_mime(mime_type);
    let random = Uuid::new_v4().simple().to_string();
    format!("{random}.{ext}")
}

pub(in crate::controllers::migration_api) fn validate_filename(
    filename: &str,
) -> std::result::Result<(), &'static str> {
    if filename.contains("..") || filename.contains('/') {
        Err("Invalid filename")
    } else {
        Ok(())
    }
}

pub(in crate::controllers::migration_api) fn parse_upload_context(
    value: Option<&str>,
) -> Option<UploadContext> {
    let raw = value.unwrap_or("profile_gallery");
    match raw {
        "profile_picture" => Some(UploadContext::ProfilePicture),
        "profile_gallery" => Some(UploadContext::ProfileGallery),
        "event_cover" => Some(UploadContext::EventCover),
        "chat_cover" => Some(UploadContext::ChatCover),
        "chat_attachment" => Some(UploadContext::ChatAttachment),
        _ => None,
    }
}

pub(in crate::controllers::migration_api) const fn is_chat_context(context: UploadContext) -> bool {
    matches!(
        context,
        UploadContext::ChatCover | UploadContext::ChatAttachment
    )
}

pub(in crate::controllers::migration_api) const fn is_upload_public(
    context: UploadContext,
) -> bool {
    matches!(
        context,
        UploadContext::ProfilePicture | UploadContext::ProfileGallery | UploadContext::EventCover
    )
}

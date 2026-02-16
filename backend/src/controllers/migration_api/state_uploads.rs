use uuid::Uuid;

use super::state_types::UploadContext;

const MAX_UPLOAD_SIZE_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;

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

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    let s: &[u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(*s))
}

fn read_u16_be(data: &[u8], offset: usize) -> Option<u16> {
    let s: &[u8; 2] = data.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(u16::from_be_bytes(*s))
}

fn read_u8(data: &[u8], offset: usize) -> Option<u8> {
    data.get(offset).copied()
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // IHDR chunk: width at byte 16, height at byte 20
    let w = read_u32_be(data, 16)?;
    let h = read_u32_be(data, 20)?;
    Some((w, h))
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // Scan for SOFn markers (0xFF 0xC0..0xCF, excluding 0xC4/0xCC)
    let mut i: usize = 2;
    while i.checked_add(9).is_some_and(|end| end < data.len()) {
        if read_u8(data, i)? != 0xFF {
            return None;
        }
        let marker = read_u8(data, i + 1)?;
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xCC {
            let h = u32::from(read_u8(data, i + 5)?) << 8 | u32::from(read_u8(data, i + 6)?);
            let w = u32::from(read_u8(data, i + 7)?) << 8 | u32::from(read_u8(data, i + 8)?);
            return Some((w, h));
        }
        let len = usize::from(read_u16_be(data, i + 2)?);
        i = i.checked_add(2)?.checked_add(len)?;
    }
    None
}

fn webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    // VP8 lossy: dimensions at bytes 26-29
    if bytes_match_at(data, 12, b"VP8 ") {
        let w = u32::from(read_u8(data, 26)?) | (u32::from(read_u8(data, 27)? & 0x3F) << 8);
        let h = u32::from(read_u8(data, 28)?) | (u32::from(read_u8(data, 29)? & 0x3F) << 8);
        return Some((w, h));
    }
    // VP8L lossless: dimensions at bytes 21-24
    if bytes_match_at(data, 12, b"VP8L") {
        let bits = u32::from(read_u8(data, 21)?)
            | (u32::from(read_u8(data, 22)?) << 8)
            | (u32::from(read_u8(data, 23)?) << 16)
            | (u32::from(read_u8(data, 24)?) << 24);
        let w = (bits & 0x3FFF) + 1;
        let h = ((bits >> 14) & 0x3FFF) + 1;
        return Some((w, h));
    }
    None
}

fn image_dimensions(data: &[u8], mime_type: &str) -> Option<(u32, u32)> {
    match mime_type {
        "image/png" => png_dimensions(data),
        "image/jpeg" => jpeg_dimensions(data),
        "image/webp" => webp_dimensions(data),
        // AVIF dimension parsing is complex (ISOBMFF); skip for now
        _ => None,
    }
}

pub(in crate::controllers::migration_api) fn validate_image_dimensions(
    data: &[u8],
    mime_type: &str,
) -> Result<(), &'static str> {
    if let Some((w, h)) = image_dimensions(data, mime_type) {
        if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
            return Err("Image dimensions exceed 8192x8192 limit");
        }
        if w == 0 || h == 0 {
            return Err("Image has zero dimensions");
        }
    }
    Ok(())
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
    if filename.is_empty()
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains('\0')
        || filename.starts_with('.')
    {
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

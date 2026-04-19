use uuid::Uuid;

use super::shared::UploadContext;

const MAX_UPLOAD_SIZE_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;

pub(in crate::api) fn is_s3_storage_configured() -> bool {
    std::env::var("GARAGE_S3_ENDPOINT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
}

pub(in crate::api) const fn max_upload_size_bytes() -> usize {
    MAX_UPLOAD_SIZE_BYTES
}

pub(in crate::api) fn allowed_upload_mime(mime_type: &str) -> bool {
    matches!(mime_type, "image/jpeg" | "image/png" | "image/webp")
}

fn bytes_match_at(data: &[u8], offset: usize, expected: &[u8]) -> bool {
    let end = offset.checked_add(expected.len());
    end.and_then(|end_idx| data.get(offset..end_idx))
        .is_some_and(|slice| slice == expected)
}

pub(in crate::api) fn validate_magic_bytes(data: &[u8], mime_type: &str) -> bool {
    match mime_type {
        "image/jpeg" => bytes_match_at(data, 0, &[0xff, 0xd8, 0xff]),
        "image/png" => bytes_match_at(data, 0, &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
        "image/webp" => {
            bytes_match_at(data, 0, &[0x52, 0x49, 0x46, 0x46])
                && bytes_match_at(data, 8, &[0x57, 0x45, 0x42, 0x50])
        }
        _ => false,
    }
}

fn read_u32_be(data: &[u8], offset: usize) -> Option<u32> {
    let s: &[u8; 4] = data.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_be_bytes(*s))
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

fn is_sofn_marker(marker: u8) -> bool {
    (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xCC
}

fn read_sofn_dimensions(data: &[u8], offset: usize) -> Option<(u32, u32)> {
    let h = u32::from(read_u8(data, offset + 5)?) << 8 | u32::from(read_u8(data, offset + 6)?);
    let w = u32::from(read_u8(data, offset + 7)?) << 8 | u32::from(read_u8(data, offset + 8)?);
    Some((w, h))
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i: usize = 2;
    // Each iteration needs at least 9 bytes from offset i (marker + SOFn header).
    while data.len().checked_sub(i).is_some_and(|rem| rem > 9) {
        let &[prefix, marker, hi, lo] = data.get(i..i + 4)?.try_into().ok()?;
        if prefix != 0xFF {
            return None;
        }
        if is_sofn_marker(marker) {
            return read_sofn_dimensions(data, i);
        }
        let len = usize::from(u16::from_be_bytes([hi, lo]));
        i = i.checked_add(2)?.checked_add(len)?;
    }
    None
}

fn webp_vp8_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let w = u32::from(read_u8(data, 26)?) | (u32::from(read_u8(data, 27)? & 0x3F) << 8);
    let h = u32::from(read_u8(data, 28)?) | (u32::from(read_u8(data, 29)? & 0x3F) << 8);
    Some((w, h))
}

fn webp_vp8l_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let bits = u32::from(read_u8(data, 21)?)
        | (u32::from(read_u8(data, 22)?) << 8)
        | (u32::from(read_u8(data, 23)?) << 16)
        | (u32::from(read_u8(data, 24)?) << 24);
    let w = (bits & 0x3FFF) + 1;
    let h = ((bits >> 14) & 0x3FFF) + 1;
    Some((w, h))
}

fn webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if bytes_match_at(data, 12, b"VP8 ") {
        return webp_vp8_dimensions(data);
    }
    if bytes_match_at(data, 12, b"VP8L") {
        return webp_vp8l_dimensions(data);
    }
    None
}

fn image_dimensions(data: &[u8], mime_type: &str) -> Option<(u32, u32)> {
    match mime_type {
        "image/png" => png_dimensions(data),
        "image/jpeg" => jpeg_dimensions(data),
        "image/webp" => webp_dimensions(data),
        _ => None,
    }
}

const fn check_dimension_bounds(w: u32, h: u32) -> Result<(), &'static str> {
    if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
        return Err("Image dimensions exceed 8192x8192 limit");
    }
    if w == 0 || h == 0 {
        return Err("Image has zero dimensions");
    }
    Ok(())
}

pub(in crate::api) fn validate_image_dimensions(
    data: &[u8],
    mime_type: &str,
) -> Result<(), &'static str> {
    if let Some((w, h)) = image_dimensions(data, mime_type) {
        check_dimension_bounds(w, h)?;
    }
    Ok(())
}

/// Strip EXIF / metadata from the uploaded image bytes.
///
/// The mobile client is *supposed* to strip EXIF before uploading,
/// but an API caller (or a mis-built client) can skip it and leak
/// GPS / device identifiers into profile pictures and attachments.
/// Re-decoding and re-encoding the pixels drops every metadata
/// chunk the original carried (EXIF, XMP, ICC, iTXt, etc.).
///
/// Quality notes:
/// * JPEG — re-encoded at quality 92, visually indistinguishable
///   from the original to human eyes; small size increase possible.
/// * PNG / WebP — re-encoded losslessly, pixel-identical.
///
/// Falls back to returning the original bytes if the decode fails
/// for any reason (truncated upload, unknown variant). Validation
/// of MIME + dimensions has already run at this point, so the
/// fallback path shouldn't realistically fire.
pub(in crate::api) fn strip_image_metadata(data: &[u8], mime_type: &str) -> Vec<u8> {
    use std::io::Cursor;

    let Ok(img) = image::load_from_memory(data) else {
        return data.to_vec();
    };

    let mut out = Vec::with_capacity(data.len());
    let result = match mime_type {
        "image/jpeg" => {
            let mut cursor = Cursor::new(&mut out);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 92).encode_image(&img)
        }
        "image/png" => img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png),
        "image/webp" => img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::WebP),
        _ => return data.to_vec(),
    };

    match result {
        Ok(()) => out,
        Err(_) => data.to_vec(),
    }
}

pub(in crate::api) fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" => "jpeg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "bin",
    }
}

pub(in crate::api) fn create_upload_filename(mime_type: &str) -> String {
    let ext = extension_for_mime(mime_type);
    let random = Uuid::new_v4().simple().to_string();
    format!("{random}.{ext}")
}

pub(in crate::api) fn validate_filename(filename: &str) -> std::result::Result<(), &'static str> {
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

pub(in crate::api) fn parse_upload_context(value: Option<&str>) -> Option<UploadContext> {
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

pub(in crate::api) const fn is_chat_context(context: UploadContext) -> bool {
    matches!(
        context,
        UploadContext::ChatCover | UploadContext::ChatAttachment
    )
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::expect_used
)]
mod tests {
    use super::strip_image_metadata;
    use std::io::Cursor;

    /// Round-trip a tiny synthesized JPEG through the `image` crate
    /// first (giving us a guaranteed-valid JPEG we can decode) and
    /// then splice an EXIF APP1 segment carrying a recognisable
    /// sentinel into it. `strip_image_metadata` must re-encode
    /// without the sentinel.
    fn jpeg_with_injected_exif_sentinel() -> (Vec<u8>, &'static [u8]) {
        let sentinel: &[u8] = b"GPS_SENTINEL_0XDEADBEEF";

        // Start with a real JPEG from the image crate so the decoder
        // definitely accepts the payload.
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([200, 50, 50]));
        let mut base = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut Cursor::new(&mut base), 90)
            .encode_image(&image::DynamicImage::ImageRgb8(img))
            .expect("encode seed jpeg");

        // Build an APP1 segment: FF E1 <len_be u16> "Exif\0\0" <payload>
        // `len_be` counts itself + Exif header + payload.
        let mut payload: Vec<u8> = Vec::new();
        payload.extend_from_slice(b"Exif\0\0");
        payload.extend_from_slice(sentinel);
        let seg_len = (2 + payload.len()) as u16; // length bytes include themselves
        let mut app1 = vec![0xff, 0xe1];
        app1.extend_from_slice(&seg_len.to_be_bytes());
        app1.extend_from_slice(&payload);

        // Inject right after the SOI (bytes 0–1 are FF D8).
        let mut with_exif = Vec::with_capacity(base.len() + app1.len());
        with_exif.extend_from_slice(&base[..2]);
        with_exif.extend_from_slice(&app1);
        with_exif.extend_from_slice(&base[2..]);
        (with_exif, sentinel)
    }

    #[test]
    fn strip_image_metadata_removes_jpeg_exif_sentinel() {
        let (input, sentinel) = jpeg_with_injected_exif_sentinel();
        assert!(
            input.windows(sentinel.len()).any(|w| w == sentinel),
            "precondition: test fixture must contain the sentinel"
        );

        let output = strip_image_metadata(&input, "image/jpeg");
        assert!(
            !output.windows(sentinel.len()).any(|w| w == sentinel),
            "strip_image_metadata must drop the EXIF sentinel"
        );
        // And the re-encoded output should still start with a valid
        // JPEG SOI marker (FF D8) so the pipeline downstream can
        // still handle it.
        assert_eq!(
            &output[..2],
            &[0xff, 0xd8],
            "re-encoded output must remain a valid JPEG"
        );
    }

    #[test]
    fn strip_image_metadata_falls_back_on_undecodable_input() {
        // Truncated garbage — the decoder rejects it, and the function
        // returns the input bytes unchanged. The upstream validation
        // chain should have rejected this before strip even runs; the
        // fallback is purely defensive.
        let garbage = vec![0xff, 0xd8, 0x00, 0x00];
        let output = strip_image_metadata(&garbage, "image/jpeg");
        assert_eq!(output, garbage);
    }
}

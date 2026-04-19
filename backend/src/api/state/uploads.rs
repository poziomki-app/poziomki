use uuid::Uuid;

use super::shared::UploadContext;

const MAX_UPLOAD_SIZE_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;
/// Decoder allocation cap. Sized to accommodate the largest legit
/// image we'd accept (8192²×4 bytes = 256 MiB) while still refusing
/// genuinely pathological decompression bombs. Anything that needs
/// more RAM than this to decode is rejected before pixels land in
/// memory, so a 40-byte PNG can't expand into multiple gigabytes.
const MAX_IMAGE_DECODE_ALLOC: u64 = 256 * 1024 * 1024;

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
/// Fails closed: if decode or re-encode fails for any reason
/// (truncated upload, unsupported sub-variant, unknown mime type
/// reaching this point) the upload is rejected rather than stored
/// with metadata intact. Validation of MIME + dimensions runs
/// before this, so a failure here means something exotic reached
/// past the front-line checks.
/// Decode `data` through the `image` crate with strict limits on
/// both dimensions and decoder allocation — this is what makes the
/// upload pipeline resistant to decompression bombs. A hostile PNG
/// of a few dozen bytes can declare a multi-gigapixel IHDR; without
/// limits the decoder happily allocates, OOMs the worker, and takes
/// the pool down. `max_image_width` / `max_image_height` short-
/// circuit at the header; `max_alloc` catches anything that slips
/// past the dimension check by declaring less-extreme pixel counts
/// backed by exotic colour depths / channels.
fn decode_with_limits(data: &[u8]) -> std::result::Result<image::DynamicImage, &'static str> {
    use std::io::Cursor;

    let mut reader = image::ImageReader::new(Cursor::new(data));
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_DECODE_ALLOC);
    reader.limits(limits);

    reader
        .with_guessed_format()
        .map_err(|_| "Image could not be decoded safely")?
        .decode()
        .map_err(|_| "Image could not be decoded safely")
}

pub(in crate::api) fn strip_image_metadata(
    data: &[u8],
    mime_type: &str,
) -> std::result::Result<Vec<u8>, &'static str> {
    use std::io::Cursor;

    let img = decode_with_limits(data)?;

    let mut out = Vec::with_capacity(data.len());
    let result = match mime_type {
        "image/jpeg" => {
            let mut cursor = Cursor::new(&mut out);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 92).encode_image(&img)
        }
        "image/png" => img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png),
        "image/webp" => img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::WebP),
        _ => return Err("Image could not be sanitized"),
    };

    result
        .map(|()| out)
        .map_err(|_| "Image could not be sanitized")
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

        let output =
            strip_image_metadata(&input, "image/jpeg").expect("strip must succeed on valid JPEG");
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
    fn strip_image_metadata_rejects_undecodable_input() {
        // Truncated garbage — the decoder rejects it, so the strip
        // function returns an error. The handler path propagates
        // this to a 400 rather than silently storing the bytes with
        // metadata intact.
        let garbage = vec![0xff, 0xd8, 0x00, 0x00];
        assert!(strip_image_metadata(&garbage, "image/jpeg").is_err());
    }

    #[test]
    fn strip_image_metadata_round_trips_small_png() {
        // Minimal 1x1 grayscale PNG (proper IDAT CRC). Same fixture
        // shape used by tests/requests/migration_contract.rs so a
        // regression that breaks PNG strip would fail here first.
        let bytes: &[u8] = &[
            0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, b'I', b'H',
            b'D', b'R', 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00,
            0x00, 0x3a, 0x7e, 0x9b, 0x55, 0x00, 0x00, 0x00, 0x0a, b'I', b'D', b'A', b'T', 0x78,
            0x9c, 0x63, 0xf8, 0x0f, 0x00, 0x01, 0x01, 0x01, 0x00, 0xb1, 0x38, 0xf6, 0x14, 0x00,
            0x00, 0x00, 0x00, b'I', b'E', b'N', b'D', 0xae, 0x42, 0x60, 0x82,
        ];
        let result = strip_image_metadata(bytes, "image/png");
        assert!(
            result.is_ok(),
            "1x1 grayscale PNG must strip OK; got {:?}",
            result.err()
        );
    }

    #[test]
    fn strip_image_metadata_round_trips_webp() {
        // Exercise the WebP encoder branch. `image::ImageFormat::WebP`
        // uses a lossless encoder by default in 0.25.x; if the build
        // features ever drop the WebP encoder we want a loud test
        // failure before prod ships uploads that silently 400.
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([0, 128, 255]));
        let mut webp = Vec::new();
        img.write_to(&mut Cursor::new(&mut webp), image::ImageFormat::WebP)
            .expect("seed webp");

        let out =
            strip_image_metadata(&webp, "image/webp").expect("strip must succeed on a valid webp");
        assert!(!out.is_empty(), "stripped webp must have bytes");
        assert!(
            out.starts_with(b"RIFF"),
            "stripped webp must remain a valid RIFF container"
        );
    }

    /// Build a tiny PNG with a valid header that declares huge
    /// dimensions — the on-wire file is only a few dozen bytes but
    /// the decoder would have to allocate gigabytes to inflate it.
    /// The strict `max_image_width` / `max_image_height` must reject
    /// before pixels touch memory.
    fn png_with_oversized_header(width: u32, height: u32) -> Vec<u8> {
        let mut out: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        // IHDR chunk: 13 bytes of data.
        out.extend_from_slice(&[0, 0, 0, 13]);
        let mut ihdr = Vec::with_capacity(17);
        ihdr.extend_from_slice(b"IHDR");
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // bit 8, colortype 2 (RGB)
        let crc = crc32(&ihdr);
        out.extend_from_slice(&ihdr);
        out.extend_from_slice(&crc.to_be_bytes());
        // IEND chunk (empty data).
        out.extend_from_slice(&[0, 0, 0, 0]);
        let iend_crc = crc32(b"IEND");
        out.extend_from_slice(b"IEND");
        out.extend_from_slice(&iend_crc.to_be_bytes());
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        // Inline CRC-32 so the test doesn't need a new dep. IEEE polynomial.
        let mut crc: u32 = 0xffff_ffff;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn strip_image_metadata_rejects_decompression_bomb() {
        // Declares a 100 000 × 100 000 PNG (≈ 40 GB decoded) in
        // a ~50-byte file. Before the limits were wired up, the
        // decoder would happily try to allocate.
        let bomb = png_with_oversized_header(100_000, 100_000);
        let result = strip_image_metadata(&bomb, "image/png");
        assert!(
            result.is_err(),
            "decompression-bomb PNG must be rejected before decode allocates pixels"
        );
    }

    #[test]
    fn strip_image_metadata_rejects_unsupported_mime() {
        // Even if decode succeeds on a crafted payload the function
        // won't encode an unknown mime. No bytes reach storage.
        let img = image::RgbImage::from_pixel(2, 2, image::Rgb([10, 20, 30]));
        let mut bytes = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut Cursor::new(&mut bytes), 80)
            .encode_image(&image::DynamicImage::ImageRgb8(img))
            .expect("encode seed");
        assert!(strip_image_metadata(&bytes, "image/heic").is_err());
    }
}

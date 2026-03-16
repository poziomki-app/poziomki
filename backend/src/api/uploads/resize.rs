use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use tokio::sync::Semaphore;

/// Max concurrent CPU-bound resize operations.
static RESIZE_SEMAPHORE: Semaphore = Semaphore::const_new(4);

/// Async entry point — acquires semaphore and offloads CPU work.
pub(in crate::api) async fn compute_thumbhash(bytes: &[u8], mime: &str) -> Result<Vec<u8>, String> {
    let _permit = RESIZE_SEMAPHORE
        .acquire()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let owned_bytes = bytes.to_vec();
    let owned_mime = mime.to_string();

    tokio::task::spawn_blocking(move || compute_thumbhash_blocking(&owned_bytes, &owned_mime))
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

fn decode_source_image(bytes: &[u8], mime: &str) -> Result<Image<'static>, String> {
    let format = match mime {
        "image/jpeg" => Some(ImageFormat::Jpeg),
        "image/png" => Some(ImageFormat::Png),
        "image/webp" => Some(ImageFormat::WebP),
        "image/avif" => Some(ImageFormat::Avif),
        _ => None,
    };

    let mut reader = ImageReader::new(Cursor::new(bytes));
    if let Some(fmt) = format {
        reader.set_format(fmt);
    }

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8000);
    limits.max_image_height = Some(8000);
    limits.max_alloc = Some(200 * 1024 * 1024);
    reader.limits(limits);

    let img = reader.decode().map_err(|e| format!("decode: {e}"))?;
    let rgba = img.to_rgba8();
    let (orig_w, orig_h) = (rgba.width(), rgba.height());
    Image::from_vec_u8(orig_w, orig_h, rgba.into_raw(), PixelType::U8x4)
        .map_err(|e| format!("src image: {e}"))
}

fn compute_thumbhash_blocking(bytes: &[u8], mime: &str) -> Result<Vec<u8>, String> {
    let src_image = decode_source_image(bytes, mime)?;
    let (orig_w, orig_h) = (src_image.width(), src_image.height());
    compute_thumbhash_from_image(&src_image, orig_w, orig_h)
}

fn fit_dimensions(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    if w <= max_dim && h <= max_dim {
        return (w, h);
    }
    if w >= h {
        let new_w = max_dim;
        let new_h = u32::try_from(u64::from(h) * u64::from(max_dim) / u64::from(w)).unwrap_or(1);
        (new_w, new_h.max(1))
    } else {
        let new_h = max_dim;
        let new_w = u32::try_from(u64::from(w) * u64::from(max_dim) / u64::from(h)).unwrap_or(1);
        (new_w.max(1), new_h)
    }
}

fn resize_rgba(src: &Image<'_>, dst_w: u32, dst_h: u32) -> Result<Vec<u8>, String> {
    let (src_w, src_h) = (src.width(), src.height());
    if src_w == dst_w && src_h == dst_h {
        return Ok(src.buffer().to_vec());
    }

    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Err("zero dimension".into());
    }

    let mut dst_image = Image::new(dst_w, dst_h, fast_image_resize::PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(src, &mut dst_image, &options)
        .map_err(|e| format!("resize: {e}"))?;

    Ok(dst_image.into_vec())
}

fn compute_thumbhash_from_image(
    src: &Image<'_>,
    orig_w: u32,
    orig_h: u32,
) -> Result<Vec<u8>, String> {
    let (tw, th) = fit_dimensions(orig_w, orig_h, 100);
    let resized = resize_rgba(src, tw, th)?;

    // thumbhash expects &[u8] of RGBA pixels
    Ok(thumbhash::rgba_to_thumb_hash(
        tw as usize,
        th as usize,
        &resized,
    ))
}

/// Re-encode image bytes to AVIF. Skips if already AVIF.
pub(super) async fn encode_avif(bytes: &[u8], mime: &str) -> Result<Vec<u8>, String> {
    if mime == "image/avif" {
        return Err("already avif".into());
    }

    let _permit = RESIZE_SEMAPHORE
        .acquire()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let owned_bytes = bytes.to_vec();
    let owned_mime = mime.to_string();

    tokio::task::spawn_blocking(move || encode_avif_blocking(&owned_bytes, &owned_mime))
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

fn encode_avif_blocking(bytes: &[u8], mime: &str) -> Result<Vec<u8>, String> {
    let src_image = decode_source_image(bytes, mime)?;
    let (w, h) = (src_image.width(), src_image.height());

    let mut buf = Vec::new();
    let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut buf, 6, 80);
    image::ImageEncoder::write_image(
        encoder,
        src_image.buffer(),
        w,
        h,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|e| format!("avif encode: {e}"))?;

    Ok(buf)
}

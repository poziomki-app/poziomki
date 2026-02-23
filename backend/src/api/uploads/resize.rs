use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use tokio::sync::Semaphore;

/// Max concurrent CPU-bound resize operations.
static RESIZE_SEMAPHORE: Semaphore = Semaphore::const_new(4);

pub(in crate::api) struct ImageVariants {
    pub(in crate::api) thumbnail: Vec<u8>,
    pub(in crate::api) standard: Vec<u8>,
    pub(in crate::api) thumbhash: Vec<u8>,
}

/// Async entry point — acquires semaphore and offloads CPU work.
pub(in crate::api) async fn generate_variants(
    bytes: &[u8],
    mime: &str,
) -> Result<ImageVariants, String> {
    let _permit = RESIZE_SEMAPHORE
        .acquire()
        .await
        .map_err(|e| format!("semaphore closed: {e}"))?;

    let owned_bytes = bytes.to_vec();
    let owned_mime = mime.to_string();

    tokio::task::spawn_blocking(move || generate_variants_blocking(&owned_bytes, &owned_mime))
        .await
        .map_err(|e| format!("spawn_blocking join error: {e}"))?
}

fn decode_source_image(bytes: &[u8], mime: &str) -> Result<Image<'static>, String> {
    let format = match mime {
        "image/jpeg" => Some(ImageFormat::Jpeg),
        "image/png" => Some(ImageFormat::Png),
        "image/webp" => Some(ImageFormat::WebP),
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

fn generate_variants_blocking(bytes: &[u8], mime: &str) -> Result<ImageVariants, String> {
    let src_image = decode_source_image(bytes, mime)?;
    let (orig_w, orig_h) = (src_image.width(), src_image.height());

    // Thumbnail: max 200px
    let (thumb_w, thumb_h) = fit_dimensions(orig_w, orig_h, 200);
    let thumbnail = resize_and_encode_webp(&src_image, thumb_w, thumb_h, 75.0)?;

    // Standard: max 800px
    let (std_w, std_h) = fit_dimensions(orig_w, orig_h, 800);
    let standard = resize_and_encode_webp(&src_image, std_w, std_h, 80.0)?;

    // Thumbhash: max 100px
    let thumbhash = compute_thumbhash(&src_image, orig_w, orig_h)?;

    Ok(ImageVariants {
        thumbnail,
        standard,
        thumbhash,
    })
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

fn validate_dimensions(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Result<(), String> {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Err("zero dimension".into());
    }
    Ok(())
}

fn resize_rgba(src: &Image<'_>, dst_w: u32, dst_h: u32) -> Result<Vec<u8>, String> {
    let (src_w, src_h) = (src.width(), src.height());
    if src_w == dst_w && src_h == dst_h {
        return Ok(src.buffer().to_vec());
    }

    validate_dimensions(src_w, src_h, dst_w, dst_h)?;

    let mut dst_image = Image::new(dst_w, dst_h, fast_image_resize::PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(src, &mut dst_image, &options)
        .map_err(|e| format!("resize: {e}"))?;

    Ok(dst_image.into_vec())
}

fn encode_webp(
    rgba_pixels: &[u8],
    width: u32,
    height: u32,
    quality: f32,
) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 {
        return Err("webp encode: zero dimension".to_string());
    }
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "webp encode: dimension overflow".to_string())?;
    if rgba_pixels.len() != expected_len {
        return Err(format!(
            "webp encode: invalid rgba buffer length {}, expected {}",
            rgba_pixels.len(),
            expected_len
        ));
    }

    let encoder = webp::Encoder::from_rgba(rgba_pixels, width, height);
    encoder
        .encode_simple(false, quality)
        .map(|encoded| encoded.to_vec())
        .map_err(|e| format!("webp encode: {e:?}"))
}

fn resize_and_encode_webp(
    src: &Image<'_>,
    dst_w: u32,
    dst_h: u32,
    quality: f32,
) -> Result<Vec<u8>, String> {
    let resized = resize_rgba(src, dst_w, dst_h)?;
    encode_webp(&resized, dst_w, dst_h, quality)
}

fn compute_thumbhash(src: &Image<'_>, orig_w: u32, orig_h: u32) -> Result<Vec<u8>, String> {
    let (tw, th) = fit_dimensions(orig_w, orig_h, 100);
    let resized = resize_rgba(src, tw, th)?;

    // thumbhash expects &[u8] of RGBA pixels
    Ok(thumbhash::rgba_to_thumb_hash(
        tw as usize,
        th as usize,
        &resized,
    ))
}

pub(in crate::api) fn variant_filename(original: &str, suffix: &str) -> String {
    let stem = original
        .rfind('.')
        .and_then(|pos| original.get(..pos))
        .unwrap_or(original);
    format!("{stem}_{suffix}.webp")
}

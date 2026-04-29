//! Image NSFW moderation via ONNX Runtime against the Marqo
//! `nsfw-image-detection-384` model (ViT-tiny, ~5M params, Apache-2.0).
//!
//! Binary classifier — output is a single sigmoid-style probability that
//! the image is NSFW. We keep the API surface tiny on purpose: the caller
//! turns the score into an [`ImageVerdict`] via the upload-side threshold,
//! mirroring the text-moderation flow but without the multi-category
//! plumbing (no need yet — Marqo only emits NSFW/SFW).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::ImageReader;
use ndarray::Array4;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use thiserror::Error;

const MODEL_FILE: &str = "model.onnx";
const INPUT_DIM: u32 = 384;

// ViT preprocessing per Marqo's pretrained_cfg
// (https://huggingface.co/Marqo/nsfw-image-detection-384/blob/main/config.json):
// mean/std are [0.5, 0.5, 0.5] — NOT ImageNet stats. Using ImageNet
// here silently shifts the input distribution and miscalibrates the
// NSFW probability.
const MEAN: [f32; 3] = [0.5, 0.5, 0.5];
const STD: [f32; 3] = [0.5, 0.5, 0.5];

#[derive(Debug, Error)]
pub enum ImageModerationError {
    #[error("image moderation model directory missing required file: {0}")]
    MissingFile(PathBuf),

    #[error("failed to load ONNX session: {0}")]
    Session(#[from] ort::Error),

    #[error("failed to decode image: {0}")]
    Decode(String),

    #[error("failed to resize image")]
    Resize,

    #[error("model returned unexpected output shape: {got:?}, expected [1, 2]")]
    BadOutputShape { got: Vec<usize> },

    #[error("image moderation engine lock poisoned")]
    LockPoisoned,
}

#[derive(Copy, Clone, Debug)]
pub struct ImageScore {
    /// Probability in `[0, 1]` that the image is NSFW.
    pub nsfw: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageVerdict {
    Allow,
    Block,
}

impl ImageVerdict {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Block => "block",
        }
    }
}

impl ImageScore {
    /// Synchronous gate threshold for user-uploaded images. 0.85 favours
    /// precision — Marqo's binary head is well-calibrated above this band
    /// (see model card AUC), and false positives on uploads are a worse
    /// product experience than letting a borderline through to async
    /// review.
    pub const BLOCK_THRESHOLD: f32 = 0.85;

    #[must_use]
    pub fn verdict(self) -> ImageVerdict {
        if self.nsfw >= Self::BLOCK_THRESHOLD {
            ImageVerdict::Block
        } else {
            ImageVerdict::Allow
        }
    }
}

pub struct ImageModerationEngine {
    // Same single-session-behind-mutex pattern as the text engine — ORT
    // intra-op threading already saturates available cores per call, so a
    // pool buys nothing on CPU.
    session: Mutex<Session>,
}

impl ImageModerationEngine {
    /// Load the Marqo ONNX model from a directory.
    ///
    /// Expected layout (produced by `scripts/marqo-export/export_onnx.py`):
    /// ```text
    ///   <dir>/model.onnx
    /// ```
    ///
    /// # Errors
    /// [`ImageModerationError::MissingFile`] if `model.onnx` isn't present;
    /// [`ImageModerationError::Session`] for ORT init failures.
    pub fn load_from_dir(dir: &Path, intra_threads: usize) -> Result<Self, ImageModerationError> {
        let model_path = dir.join(MODEL_FILE);
        if !model_path.is_file() {
            return Err(ImageModerationError::MissingFile(model_path));
        }

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort::Error::from)?
            .with_intra_threads(intra_threads)
            .map_err(ort::Error::from)?
            .with_memory_pattern(false)
            .map_err(ort::Error::from)?
            .commit_from_file(&model_path)?;

        Ok(Self {
            session: Mutex::new(session),
        })
    }

    /// Score raw image bytes. Caller is responsible for `spawn_blocking`.
    ///
    /// # Errors
    /// Decode failures become [`ImageModerationError::Decode`]; downstream
    /// resize/inference failures map to their respective variants.
    #[allow(clippy::significant_drop_tightening)]
    pub fn score(&self, bytes: &[u8]) -> Result<ImageScore, ImageModerationError> {
        let rgb = decode_to_rgb(bytes)?;
        let resized = resize_to_input(&rgb)?;
        let tensor = to_chw_tensor(&resized)?;

        let logits_owned: Vec<f32> = {
            let mut session = self
                .session
                .lock()
                .map_err(|_| ImageModerationError::LockPoisoned)?;
            let outputs = session.run(ort::inputs!["input" => tensor])?;
            // The exporter pins the output name to `output` (see script).
            let value = outputs
                .get("output")
                .ok_or(ImageModerationError::BadOutputShape { got: vec![] })?;
            let (shape, logits) = value.try_extract_tensor::<f32>()?;
            let dims: Vec<usize> = shape
                .iter()
                .map(|&d| usize::try_from(d).unwrap_or(0))
                .collect();
            if dims != [1, 2] {
                return Err(ImageModerationError::BadOutputShape { got: dims });
            }
            logits.to_vec()
        };

        // timm `vit_tiny_patch16_384` finetune from Marqo emits class
        // order `[NSFW, SFW]` — softmax index 0 is the NSFW probability.
        let nsfw = softmax2_first(
            logits_owned.first().copied().unwrap_or(0.0),
            logits_owned.get(1).copied().unwrap_or(0.0),
        );
        Ok(ImageScore { nsfw })
    }
}

fn decode_to_rgb(bytes: &[u8]) -> Result<image::RgbImage, ImageModerationError> {
    let cursor = std::io::Cursor::new(bytes);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| ImageModerationError::Decode(e.to_string()))?;
    let dynamic = reader
        .decode()
        .map_err(|e| ImageModerationError::Decode(e.to_string()))?;
    Ok(dynamic.to_rgb8())
}

fn resize_to_input(src: &image::RgbImage) -> Result<Vec<u8>, ImageModerationError> {
    let (w, h) = src.dimensions();
    let src_image = FirImage::from_vec_u8(w, h, src.as_raw().clone(), PixelType::U8x3)
        .map_err(|_| ImageModerationError::Resize)?;
    let mut dst = FirImage::new(INPUT_DIM, INPUT_DIM, PixelType::U8x3);
    let mut resizer = Resizer::new();
    // HF pretrained_cfg specifies bicubic resampling. CatmullRom is the
    // canonical bicubic kernel (Keys cubic, a=-0.5) — the same one PIL
    // uses for `Image.BICUBIC`, which is the de-facto reference for timm
    // preprocessing. fast_image_resize defaults to Lanczos3, which would
    // bias the input distribution relative to training.
    let opts = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::CatmullRom));
    resizer
        .resize(&src_image, &mut dst, &opts)
        .map_err(|_| ImageModerationError::Resize)?;
    Ok(dst.into_vec())
}

fn to_chw_tensor(rgb: &[u8]) -> Result<Tensor<f32>, ImageModerationError> {
    let dim = INPUT_DIM as usize;
    let mut chw = vec![0f32; 3 * dim * dim];
    let plane = dim * dim;
    for (idx, chunk) in rgb.chunks_exact(3).enumerate() {
        for c in 0..3 {
            let raw = f32::from(chunk.get(c).copied().unwrap_or(0)) / 255.0;
            let normalized =
                (raw - MEAN.get(c).copied().unwrap_or(0.0)) / STD.get(c).copied().unwrap_or(1.0);
            if let Some(slot) = chw.get_mut(c * plane + idx) {
                *slot = normalized;
            }
        }
    }
    let arr = Array4::<f32>::from_shape_vec((1, 3, dim, dim), chw)
        .map_err(|_| ImageModerationError::Resize)?;
    Tensor::from_array(arr).map_err(ImageModerationError::Session)
}

#[inline]
fn softmax2_first(a: f32, b: f32) -> f32 {
    let m = a.max(b);
    let ea = (a - m).exp();
    let eb = (b - m).exp();
    ea / (ea + eb)
}

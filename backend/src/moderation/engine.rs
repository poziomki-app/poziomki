use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ndarray::{Array2, Axis};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use thiserror::Error;
use tokenizers::Tokenizer;

use super::scores::{Category, Scores};

/// Hard cap on input tokens sent to the model. The `RoBERTa` encoder
/// supports 512 positions; we clamp lower to keep CPU inference latency
/// predictable on long user text. Anything above is truncated at the
/// tokenizer.
const MAX_SEQ_LEN: usize = 256;

/// Filename of the int8 ONNX artifact produced by
/// `scripts/guard-bench/export_onnx.py`.
const MODEL_FILE: &str = "model_quantized.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";

#[derive(Debug, Error)]
pub enum ModerationError {
    #[error("moderation model directory missing required file: {0}")]
    MissingFile(PathBuf),

    #[error("failed to load ONNX session: {0}")]
    Session(#[from] ort::Error),

    #[error("failed to load tokenizer: {0}")]
    Tokenizer(String),

    #[error("tokenization failed: {0}")]
    Encode(String),

    #[error("model returned unexpected output shape: {got:?}, expected [_, 5]")]
    BadOutputShape { got: Vec<usize> },

    #[error("failed to build input tensor")]
    TensorBuild,

    #[error("moderation engine lock poisoned")]
    LockPoisoned,
}

/// Loaded inference engine: ONNX session + tokenizer, held for the lifetime
/// of the process. Inference is CPU-bound and blocking; call the sync API
/// from a `tokio::task::spawn_blocking` in async contexts.
pub struct ModerationEngine {
    // `ort::Session::run` requires `&mut self` (rc.12), so we serialize
    // access behind a Mutex. For higher concurrency, replace with a pool of
    // sessions — but on CPU with ORT intra-op threading, a single session
    // already saturates available cores, and blocking callers go through
    // `tokio::task::spawn_blocking` anyway.
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    pad_id: i64,
}

impl ModerationEngine {
    /// Load the quantized Bielik-Guard model and its tokenizer from a
    /// directory populated by `scripts/guard-bench/export_onnx.py`.
    ///
    /// `intra_threads` caps the CPU threads used per inference call. For a
    /// shared web server, 1–2 is usually right — scale concurrency via the
    /// number of tokio blocking workers, not ORT's thread pool.
    ///
    /// # Errors
    /// Returns [`ModerationError`] when required files are missing, the ONNX
    /// session fails to build, or the tokenizer file is malformed.
    pub fn load_from_dir(dir: &Path, intra_threads: usize) -> Result<Self, ModerationError> {
        let model_path = dir.join(MODEL_FILE);
        let tokenizer_path = dir.join(TOKENIZER_FILE);
        if !model_path.is_file() {
            return Err(ModerationError::MissingFile(model_path));
        }
        if !tokenizer_path.is_file() {
            return Err(ModerationError::MissingFile(tokenizer_path));
        }

        // Builder setter errors are tagged `ort::Error<SessionBuilder>`;
        // erase the tag with `ort::Error::from` so `?` converts via
        // `#[from] ort::Error<()>` on `ModerationError`.
        // Tuned for the prod OVH containers (api 512 MB, worker 256 MB).
        //
        // `with_memory_pattern(false)` disables ORT's buffer-size caching
        // that otherwise allocates worst-case scratch buffers on the first
        // inferences and holds them indefinitely — observed warmup RSS of
        // ~370 MB on this model with pattern on, vs. a steady-state of
        // ~100 MB. The cost is ~10–15 % higher per-inference latency on
        // varying input lengths, which at sub-10 ms p50 is well within
        // budget for our use case.
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort::Error::from)?
            .with_intra_threads(intra_threads)
            .map_err(ort::Error::from)?
            .with_memory_pattern(false)
            .map_err(ort::Error::from)?
            .commit_from_file(&model_path)?;

        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| ModerationError::Tokenizer(e.to_string()))?;
        // Enable built-in truncation; max_length matches MAX_SEQ_LEN so we
        // never have to re-check on the Rust side.
        let trunc = tokenizers::TruncationParams {
            max_length: MAX_SEQ_LEN,
            ..Default::default()
        };
        tokenizer
            .with_truncation(Some(trunc))
            .map_err(|e| ModerationError::Tokenizer(e.to_string()))?;

        // Resolve the pad token id from the tokenizer (RoBERTa: id 1, "<pad>").
        let pad_id = i64::from(tokenizer.token_to_id("<pad>").unwrap_or(1));

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            pad_id,
        })
    }

    /// Score a single input string.
    ///
    /// # Errors
    /// Returns [`ModerationError`] on tokenization or inference failure.
    pub fn score(&self, text: &str) -> Result<Scores, ModerationError> {
        let out = self.score_batch(&[text])?;
        out.into_iter()
            .next()
            .ok_or(ModerationError::BadOutputShape { got: vec![0, 5] })
    }

    /// Score a batch of inputs in one inference call.
    ///
    /// Attention-mask padding means per-sample scores are independent of
    /// batch composition. Empty batch returns an empty vector without
    /// touching the session.
    ///
    /// # Errors
    /// Returns [`ModerationError`] on tokenization, tensor construction,
    /// inference, or output-shape failures.
    #[allow(clippy::significant_drop_tightening)]
    pub fn score_batch(&self, texts: &[&str]) -> Result<Vec<Scores>, ModerationError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| ModerationError::Encode(e.to_string()))?;

        let batch = encodings.len();
        let seq_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);
        if seq_len == 0 {
            // Degenerate case — all inputs empty. Return neutral scores
            // rather than calling into ORT with a zero-sized tensor.
            return Ok(vec![
                Scores {
                    self_harm: 0.0,
                    hate: 0.0,
                    vulgar: 0.0,
                    sex: 0.0,
                    crime: 0.0,
                };
                batch
            ]);
        }

        let mut ids = Vec::with_capacity(batch * seq_len);
        let mut mask = Vec::with_capacity(batch * seq_len);
        for enc in &encodings {
            let enc_ids = enc.get_ids();
            let enc_mask = enc.get_attention_mask();
            let used = enc_ids.len();
            ids.extend(enc_ids.iter().map(|&i| i64::from(i)));
            mask.extend(enc_mask.iter().map(|&m| i64::from(m)));
            for _ in used..seq_len {
                ids.push(self.pad_id);
                mask.push(0);
            }
        }

        let ids_arr = Array2::<i64>::from_shape_vec((batch, seq_len), ids)
            .map_err(|_| ModerationError::TensorBuild)?;
        let mask_arr = Array2::<i64>::from_shape_vec((batch, seq_len), mask)
            .map_err(|_| ModerationError::TensorBuild)?;

        let ids_tensor = Tensor::from_array(ids_arr)?;
        let mask_tensor = Tensor::from_array(mask_arr)?;

        // Hold the lock only for the inference call. The returned
        // `SessionOutputs` borrow from the session, so we copy logits out
        // into an owned Vec before dropping the guard.
        let logits_owned: Vec<f32> = {
            let mut session = self
                .session
                .lock()
                .map_err(|_| ModerationError::LockPoisoned)?;
            let outputs = session.run(ort::inputs![
                "input_ids" => ids_tensor,
                "attention_mask" => mask_tensor,
            ])?;
            let logits_value = outputs
                .get("logits")
                .ok_or(ModerationError::BadOutputShape { got: vec![] })?;
            let (shape, logits) = logits_value.try_extract_tensor::<f32>()?;
            let dims: Vec<usize> = shape
                .iter()
                .map(|&d| usize::try_from(d).unwrap_or(0))
                .collect();
            if dims.len() != 2
                || dims.first().copied() != Some(batch)
                || dims.get(1).copied() != Some(5)
            {
                return Err(ModerationError::BadOutputShape { got: dims });
            }
            logits.to_vec()
        };

        let view =
            ndarray::ArrayView2::<f32>::from_shape((batch, 5), &logits_owned).map_err(|_| {
                ModerationError::BadOutputShape {
                    got: vec![batch, 5],
                }
            })?;

        let mut out = Vec::with_capacity(batch);
        for row in view.axis_iter(Axis(0)) {
            out.push(Scores {
                self_harm: sigmoid(row.get(Category::SelfHarm as usize).copied().unwrap_or(0.0)),
                hate: sigmoid(row.get(Category::Hate as usize).copied().unwrap_or(0.0)),
                vulgar: sigmoid(row.get(Category::Vulgar as usize).copied().unwrap_or(0.0)),
                sex: sigmoid(row.get(Category::Sex as usize).copied().unwrap_or(0.0)),
                crime: sigmoid(row.get(Category::Crime as usize).copied().unwrap_or(0.0)),
            });
        }
        Ok(out)
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

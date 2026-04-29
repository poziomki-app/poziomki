//! End-to-end regression test for the image NSFW engine.
//!
//! Loads the real Marqo ONNX model from `IMAGE_MODERATION_MODEL_PATH`
//! (or skips if unset) and verifies the Rust preprocessing + inference
//! path matches the Python timm reference within a small tolerance.
//!
//! Generate fixtures + reference scores (one-time) with
//! `scripts/marqo-export/run_reference.py`, then:
//!
//! ```text
//!   IMAGE_MODERATION_MODEL_PATH=$HOME/models/marqo-nsfw-onnx \
//!   IMAGE_MODERATION_FIXTURES=/tmp/nsfw-e2e/fixtures \
//!     cargo test --test image_moderation -- --nocapture
//! ```

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::float_cmp)]
#![allow(clippy::items_after_statements)]

use std::path::PathBuf;

use poziomki_backend::moderation::ImageModerationEngine;

#[derive(serde::Deserialize)]
struct RefEntry {
    path: String,
    nsfw: f32,
}

#[test]
fn rust_inference_matches_python_timm_reference() {
    let Ok(model_dir) = std::env::var("IMAGE_MODERATION_MODEL_PATH") else {
        eprintln!("IMAGE_MODERATION_MODEL_PATH unset — skipping image-moderation E2E test");
        return;
    };
    let Ok(fixtures_dir) = std::env::var("IMAGE_MODERATION_FIXTURES") else {
        eprintln!("IMAGE_MODERATION_FIXTURES unset — skipping image-moderation E2E test");
        return;
    };

    let engine = ImageModerationEngine::load_from_dir(&PathBuf::from(&model_dir), 4)
        .expect("failed to load image moderation engine");

    let ref_path = PathBuf::from(&fixtures_dir).join("reference.json");
    let raw = std::fs::read_to_string(&ref_path).expect("read reference.json");
    let refs: std::collections::BTreeMap<String, RefEntry> =
        serde_json::from_str(&raw).expect("parse reference.json");

    // Tolerance: Rust resize (fast_image_resize CatmullRom) and PIL
    // BICUBIC are both Keys cubic but the internal pixel-grid math
    // differs by sub-pixel offsets, which is most visible on smooth
    // gradients downscaled by large factors. Empirically ≤0.05 absolute
    // on the softmax probability after fixing mean/std. >0.05 indicates
    // a real preprocessing bug (mean/std swap, channel swap, missing
    // /255, etc) — for reference, the original ImageNet-stats bug
    // produced 0.08–0.14 deltas on these fixtures.
    const TOLERANCE: f32 = 0.05;

    let mut max_delta: f32 = 0.0;
    let mut failures = Vec::new();
    for (name, entry) in &refs {
        let bytes = std::fs::read(&entry.path).expect("read fixture image");
        let score = engine.score(&bytes).expect("score fixture");
        let delta = (score.nsfw - entry.nsfw).abs();
        max_delta = max_delta.max(delta);
        println!(
            "fixture={name} python_nsfw={:.6} rust_nsfw={:.6} delta={:.6}",
            entry.nsfw, score.nsfw, delta
        );
        if delta >= TOLERANCE {
            failures.push(format!(
                "{name}: rust={:.6} python={:.6} delta={:.6}",
                score.nsfw, entry.nsfw, delta
            ));
        }
    }
    println!("max_delta across fixtures: {max_delta:.6}");
    assert!(
        failures.is_empty(),
        "fixtures exceeded tolerance {TOLERANCE}: {failures:?}"
    );
}

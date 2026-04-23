//! End-to-end smoke test for the moderation engine.
//!
//! Loads the real int8 Bielik-Guard ONNX model from
//! `BIELIK_GUARD_ONNX_DIR` (or skips if unset) and asserts that each
//! category in the Polish sample set is correctly flagged or left alone.
//! Prints p50/p95 single-shot latency + batched throughput.
//!
//! Run:
//!   BIELIK_GUARD_ONNX_DIR=$HOME/models/bielik-guard-onnx \
//!     cargo test --test moderation -- --nocapture

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::panic)]
#![allow(clippy::float_cmp)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_panics_doc)]

use std::path::PathBuf;
use std::time::Instant;

use poziomki_backend::moderation::{Category, ModerationEngine, Scores, Thresholds, Verdict};

/// (expected top category or "safe", text).
/// "safe" means: every sigmoid score should be below the bio threshold.
/// A concrete category means: that category's score should be the model's
/// highest AND above 0.5.
fn samples() -> Vec<(&'static str, &'static str)> {
    vec![
        // --- safe ---
        (
            "safe",
            "Cześć, jestem Ania, lubię wspinaczkę i jazz. Szukam ludzi do wspólnych wyjść.",
        ),
        (
            "safe",
            "Poszukuję ekipy na weekendowe wyjazdy rowerowe w okolicach Krakowa.",
        ),
        (
            "safe",
            "Piszę książkę o historii Polski, chętnie poznam kogoś z podobnymi zainteresowaniami.",
        ),
        (
            "safe",
            "Dzisiaj zrobiłem świetny obiad — polecam przepis na pierogi ruskie od babci.",
        ),
        (
            "safe",
            "Ktoś jeszcze ogląda nowy sezon? Bez spoilerów proszę!",
        ),
        (
            "safe",
            "Mam 34 lata, dwójka dzieci, praca w IT, w wolnym czasie biegam.",
        ),
        // --- hate (model often splits hate/vulgar — accept either as "flagged") ---
        ("hate", "Nienawidzę tych ciapatych, niech giną."),
        ("hate", "Żydzi rządzą światem i okradają Polskę."),
        ("hate", "Baby są głupie i nadają się tylko do kuchni."),
        // --- vulgar ---
        ("vulgar", "Ty kurwo jebana, spierdalaj stąd."),
        ("vulgar", "Ale z ciebie chuj, serio."),
        ("vulgar", "Co za pojebany dzień, kurwa mać."),
        // --- sex ---
        ("sex", "Szukam faceta do seksu bez zobowiązań, DM."),
        ("sex", "Wyślę ci nagie zdjęcia jak napiszesz."),
        ("sex", "Lubię ostry seks analny, ktoś chętny?"),
        // --- self-harm (the high-stakes category we MUST catch) ---
        (
            "self-harm",
            "Nie chce mi się już żyć, wszystko straciło sens.",
        ),
        ("self-harm", "Mam ochotę się pociąć, żeby przestać to czuć."),
        (
            "self-harm",
            "Codziennie myślę o samobójstwie, nie mam siły.",
        ),
        ("self-harm", "Tnę się od dwóch lat, nikt nie wie."),
        // --- crime ---
        ("crime", "Sprzedam metę, 200 zł gram, Warszawa."),
        (
            "crime",
            "Szukam kogoś do włamania do mieszkania na Mokotowie.",
        ),
        ("crime", "Kupię broń bez pozwolenia, płacę gotówką."),
    ]
}

fn model_dir() -> Option<PathBuf> {
    std::env::var_os("BIELIK_GUARD_ONNX_DIR").map(PathBuf::from)
}

fn cat_from(name: &str) -> Option<Category> {
    Category::ALL.into_iter().find(|c| c.as_str() == name)
}

fn top(scores: &Scores) -> (Category, f32) {
    Category::ALL.into_iter().map(|c| (c, scores.get(c))).fold(
        (Category::SelfHarm, f32::MIN),
        |acc, (c, s)| if s > acc.1 { (c, s) } else { acc },
    )
}

#[test]
fn scores_polish_sample_set_correctly() {
    let Some(dir) = model_dir() else {
        eprintln!("skipping: set BIELIK_GUARD_ONNX_DIR to run this test");
        return;
    };
    let engine = ModerationEngine::load_from_dir(&dir, 2).expect("load moderation engine");

    let rows = samples();
    let mut failures: Vec<String> = Vec::new();
    let mut single_times_ms: Vec<f64> = Vec::with_capacity(rows.len());

    println!("\n{:12} {:12} {:5}  text", "expected", "got", "score");
    for (expected, text) in &rows {
        let started = Instant::now();
        let scores = engine.score(text).expect("inference");
        single_times_ms.push(started.elapsed().as_secs_f64() * 1000.0);

        let (top_cat, top_score) = top(&scores);
        let snippet = if text.chars().count() > 60 {
            let truncated: String = text.chars().take(57).collect();
            format!("{}...", truncated)
        } else {
            (*text).to_string()
        };
        println!(
            "{:12} {:12} {:.2}  {}",
            expected,
            top_cat.as_str(),
            top_score,
            snippet
        );

        let flagged = scores.flagged(&Thresholds::BIO);
        let verdict = scores.verdict(&Thresholds::BIO);

        match *expected {
            "safe" => {
                if !flagged.is_empty() {
                    failures.push(format!(
                        "FALSE POSITIVE on safe text: {:?}  flagged={:?}",
                        text, flagged
                    ));
                }
                if verdict != Verdict::Allow {
                    failures.push(format!(
                        "expected Verdict::Allow for safe text, got {:?}: {:?}",
                        verdict, text
                    ));
                }
            }
            target_name => {
                let Some(target) = cat_from(target_name) else {
                    failures.push(format!("unknown expected category: {}", target_name));
                    continue;
                };
                // We require the target category to be above 0.5 OR the
                // top category to be a closely-related harm class. For
                // hate/vulgar we accept either since those correlate.
                let target_hit = scores.get(target) >= 0.5;
                let acceptable_confusion = matches!(
                    (target, top_cat),
                    (Category::Hate, Category::Vulgar) | (Category::Vulgar, Category::Hate)
                );
                if !(target_hit || acceptable_confusion && top_score >= 0.5) {
                    failures.push(format!(
                        "FALSE NEGATIVE on {}: top={}({:.2}) target={:.2}  text={:?}",
                        target_name,
                        top_cat.as_str(),
                        top_score,
                        scores.get(target),
                        text,
                    ));
                }
                if verdict == Verdict::Allow {
                    failures.push(format!(
                        "expected non-Allow verdict for {} text, got Allow: {:?}",
                        target_name, text
                    ));
                }
            }
        }
    }

    // Latency stats on single-shot path.
    let mut sorted = single_times_ms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = sorted[sorted.len() / 2];
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
    let mean = single_times_ms.iter().sum::<f64>() / single_times_ms.len() as f64;

    // Batched throughput.
    let texts: Vec<&str> = rows.iter().map(|(_, t)| *t).collect();
    // warmup
    engine.score_batch(&texts[..4]).expect("warmup");
    let t = Instant::now();
    let _batched = engine.score_batch(&texts).expect("batched inference");
    let batched_total_ms = t.elapsed().as_secs_f64() * 1000.0;
    let per_msg = batched_total_ms / texts.len() as f64;

    println!(
        "\nlatency single: p50={:.1}ms p95={:.1}ms mean={:.1}ms",
        p50, p95, mean
    );
    println!(
        "batched ({} msgs): total={:.1}ms  per-msg={:.1}ms",
        texts.len(),
        batched_total_ms,
        per_msg
    );

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("  - {}", f);
        }
        panic!(
            "{} moderation assertions failed (see stderr)",
            failures.len()
        );
    }
}

#[test]
fn empty_batch_returns_empty() {
    let Some(dir) = model_dir() else {
        return;
    };
    let engine = ModerationEngine::load_from_dir(&dir, 1).expect("load");
    let out = engine.score_batch(&[]).expect("empty batch");
    assert!(out.is_empty());
}

#[test]
fn very_long_input_is_truncated_not_errored() {
    let Some(dir) = model_dir() else {
        return;
    };
    let engine = ModerationEngine::load_from_dir(&dir, 1).expect("load");
    let long = "kurwa ".repeat(500);
    let scores = engine.score(&long).expect("long inference");
    // Should flag vulgar.
    assert!(
        scores.vulgar > 0.5,
        "vulgar score on long vulgar input should be >0.5, got {}",
        scores.vulgar
    );
}

#[test]
fn init_from_env_populates_global_singleton() {
    // Exercises the boot path used by `run_api_server` /
    // `run_outbox_worker_process`: `MODERATION_MODEL_PATH` set →
    // `shared()` returns Some. The caller is expected to set this env
    // var alongside `BIELIK_GUARD_ONNX_DIR` (they typically point at the
    // same directory). Skipped if either is unset.
    if model_dir().is_none() || std::env::var_os("MODERATION_MODEL_PATH").is_none() {
        eprintln!(
            "skipping: set both BIELIK_GUARD_ONNX_DIR and MODERATION_MODEL_PATH \
             to the same path to run this test"
        );
        return;
    }
    poziomki_backend::moderation::init_from_env().expect("init");
    let engine = poziomki_backend::moderation::shared()
        .expect("shared engine must be Some after successful init");
    let scores = engine.score("Cześć, jak się masz?").expect("score");
    assert_eq!(
        scores.verdict(&Thresholds::BIO),
        Verdict::Allow,
        "neutral greeting must be Allow; scores={:?}",
        scores,
    );
}

#[test]
fn thresholds_verdict_enforces_block_before_flag() {
    let scores = Scores {
        self_harm: 0.95,
        hate: 0.0,
        vulgar: 0.0,
        sex: 0.0,
        crime: 0.0,
    };
    assert_eq!(scores.verdict(&Thresholds::BIO), Verdict::Block);
    assert_eq!(scores.verdict(&Thresholds::CHAT), Verdict::Block);

    let below_block = Scores {
        self_harm: 0.6,
        ..scores
    };
    // CHAT block threshold for self-harm is 0.9; flag is 0.5; so Flag.
    assert_eq!(below_block.verdict(&Thresholds::CHAT), Verdict::Flag);

    let allow = Scores {
        self_harm: 0.3,
        ..scores
    };
    assert_eq!(allow.verdict(&Thresholds::CHAT), Verdict::Allow);
}

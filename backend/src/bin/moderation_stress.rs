//! Stress + stability harness for the moderation engine, calibrated to the
//! prod OVH container limits (api 512 MB / 1 CPU, worker 256 MB / 0.5 CPU).
//!
//! Runs four phases and prints a machine-parseable summary:
//!   1. cold-load footprint (RSS before/after model load)
//!   2. sustained single-threaded loop (leak check across 10 000 inferences)
//!   3. concurrent load (N blocking workers via `spawn_blocking`)
//!   4. final memory check
//!
//! Intended invocation under cgroup limits:
//!   systemd-run --user --scope -p MemoryMax=256M -p CPUQuota=50% -- \
//!     ./target/release/moderation-stress --concurrency 2 --iters 10000
//!
//! `MODERATION_MODEL_PATH` must point at the ONNX directory produced by
//! `scripts/guard-bench/export_onnx.py`.

#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::missing_docs_in_private_items)]

use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use poziomki_backend::moderation::{self, ModerationEngine, Scores};

const SAMPLES: &[&str] = &[
    "Cześć, jestem Ania, lubię wspinaczkę i jazz.",
    "Poszukuję ekipy na weekendowe wyjazdy rowerowe.",
    "Dzisiaj zrobiłem świetny obiad — polecam pierogi.",
    "Mam 34 lata, dwójka dzieci, w wolnym czasie biegam.",
    "Ty kurwo jebana, spierdalaj stąd.",
    "Co za pojebany dzień, kurwa mać.",
    "Nienawidzę tych ciapatych, niech giną.",
    "Żydzi rządzą światem i okradają Polskę.",
    "Szukam faceta do seksu bez zobowiązań, DM.",
    "Lubię ostry seks analny, ktoś chętny?",
    "Nie chce mi się już żyć, wszystko straciło sens.",
    "Mam ochotę się pociąć, żeby przestać to czuć.",
    "Sprzedam metę, 200 zł gram, Warszawa.",
    "Szukam kogoś do włamania do mieszkania na Mokotowie.",
    // A realistic chat-length sample (~400 chars) to stress tokeniser + padding.
    "Hej, dawno się nie widzieliśmy, jak tam u ciebie? Słyszałem, że zmieniłaś pracę — \
     gratulacje, trzymam kciuki za nową rolę. U mnie bez większych zmian, próbuję \
     wyrobić trochę spokojniejszy rytm, mniej siedzenia przy kompie wieczorami, więcej \
     wyjść na rower. W weekend jedziemy z chłopakami na Kaszuby, jakby coś się zmieniło \
     w Twoich planach, daj znać, chętnie nadgonimy przy piwie po powrocie.",
];

struct RssKb(u64);

impl RssKb {
    fn read() -> Self {
        let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb = rest
                    .trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse::<u64>()
                    .unwrap_or(0);
                return Self(kb);
            }
        }
        Self(0)
    }

    const fn mb(&self) -> f64 {
        self.0 as f64 / 1024.0
    }
}

#[derive(Default)]
struct LatencyStats {
    samples_us: Vec<u64>,
}

impl LatencyStats {
    fn record(&mut self, elapsed: Duration) {
        self.samples_us.push(elapsed.as_micros() as u64);
    }

    fn percentile(&mut self, p: f64) -> f64 {
        if self.samples_us.is_empty() {
            return 0.0;
        }
        self.samples_us.sort_unstable();
        let idx = ((self.samples_us.len() as f64 - 1.0) * p).round() as usize;
        self.samples_us[idx.min(self.samples_us.len() - 1)] as f64 / 1000.0
    }

    fn mean_ms(&self) -> f64 {
        if self.samples_us.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.samples_us.iter().sum();
        sum as f64 / self.samples_us.len() as f64 / 1000.0
    }
}

fn parse_args() -> (usize, usize, usize) {
    // --iters N --concurrency N --threads N
    let mut iters = 10_000usize;
    let mut concurrency = 2usize;
    let mut intra_threads = 1usize;
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        // Every flag below consumes the next argv slot — bail cleanly if
        // the caller left it dangling (e.g. `--iters` as the last arg).
        let value = args.get(i + 1);
        match args[i].as_str() {
            "--iters" => {
                if let Some(v) = value {
                    iters = v.parse().unwrap_or(iters);
                }
                i += 2;
            }
            "--concurrency" => {
                if let Some(v) = value {
                    // max(1): 0 would divide-by-zero in the phase loops.
                    concurrency = v.parse::<usize>().unwrap_or(concurrency).max(1);
                }
                i += 2;
            }
            "--threads" => {
                if let Some(v) = value {
                    intra_threads = v.parse::<usize>().unwrap_or(intra_threads).max(1);
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    (iters, concurrency, intra_threads)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let (iters, concurrency, intra_threads) = parse_args();

    println!("=== moderation stress bench ===");
    println!(
        "iters={iters}  concurrency={concurrency}  intra_threads={intra_threads}  \
         sample_set={}",
        SAMPLES.len()
    );

    // --- Phase 1: cold-load footprint ---
    let baseline = RssKb::read();
    println!("\n[1] baseline RSS: {:.1} MB", baseline.mb());

    let model_path = std::env::var("MODERATION_MODEL_PATH")
        .expect("MODERATION_MODEL_PATH env var must point at ONNX dir");
    let load_started = Instant::now();
    let engine = Arc::new(
        ModerationEngine::load_from_dir(std::path::Path::new(&model_path), intra_threads)
            .expect("load engine"),
    );
    let load_ms = load_started.elapsed().as_secs_f64() * 1000.0;
    let after_load = RssKb::read();
    // Also seed the global for parity with prod init path (env var is
    // already set from the caller; we just read it back).
    let _ = moderation::init_from_env();
    println!(
        "    load: {load_ms:.0} ms   RSS after load: {:.1} MB   delta: {:+.1} MB",
        after_load.mb(),
        after_load.mb() - baseline.mb()
    );

    // Warmup: first few inferences trigger lazy ORT init and tokenizer caches.
    for s in SAMPLES.iter().take(4) {
        let _ = engine.score(s);
    }
    let after_warmup = RssKb::read();
    println!(
        "    RSS after warmup: {:.1} MB   delta-from-load: {:+.1} MB",
        after_warmup.mb(),
        after_warmup.mb() - after_load.mb()
    );

    // --- Phase 2: sustained single-threaded leak check ---
    println!("\n[2] sustained single-threaded: {iters} iterations");
    let pre_sustained = RssKb::read();
    let mut stats = LatencyStats::default();
    let phase_started = Instant::now();
    let mut checkpoints = Vec::new();
    let checkpoint_every = iters / 10;
    for i in 0..iters {
        let text = SAMPLES[i % SAMPLES.len()];
        let t = Instant::now();
        let _scores: Scores = engine.score(text).expect("score");
        stats.record(t.elapsed());
        if checkpoint_every > 0 && (i + 1) % checkpoint_every == 0 {
            checkpoints.push((i + 1, RssKb::read().mb()));
        }
    }
    let phase_elapsed = phase_started.elapsed();
    let post_sustained = RssKb::read();
    let throughput = iters as f64 / phase_elapsed.as_secs_f64();

    println!(
        "    elapsed: {:.2}s   throughput: {throughput:.1} msg/s",
        phase_elapsed.as_secs_f64()
    );
    println!(
        "    latency mean={:.2}ms  p50={:.2}ms  p95={:.2}ms  p99={:.2}ms  p99.9={:.2}ms",
        stats.mean_ms(),
        stats.percentile(0.50),
        stats.percentile(0.95),
        stats.percentile(0.99),
        stats.percentile(0.999),
    );
    println!(
        "    RSS pre: {:.1} MB   post: {:.1} MB   delta: {:+.1} MB  (leak check)",
        pre_sustained.mb(),
        post_sustained.mb(),
        post_sustained.mb() - pre_sustained.mb()
    );
    println!("    checkpoints (iter -> MB):");
    for (i, mb) in &checkpoints {
        println!("      {i:>7}: {mb:.1}");
    }

    // --- Phase 3: concurrent load via spawn_blocking ---
    println!(
        "\n[3] concurrent load: {concurrency} workers × {} iters each",
        iters / concurrency
    );
    let per_worker = iters / concurrency;
    let pre_concurrent = RssKb::read();
    let concurrent_started = Instant::now();
    let mut handles = Vec::with_capacity(concurrency);
    for w in 0..concurrency {
        let engine = Arc::clone(&engine);
        handles.push(tokio::spawn(async move {
            let mut wstats = LatencyStats::default();
            for i in 0..per_worker {
                let text = SAMPLES[(w * 31 + i) % SAMPLES.len()];
                let engine = Arc::clone(&engine);
                let t = Instant::now();
                let _scores = tokio::task::spawn_blocking(move || engine.score(text))
                    .await
                    .expect("join")
                    .expect("score");
                wstats.record(t.elapsed());
            }
            wstats
        }));
    }
    let mut combined = LatencyStats::default();
    for h in handles {
        let s = h.await.expect("worker join");
        combined.samples_us.extend(s.samples_us);
    }
    let concurrent_elapsed = concurrent_started.elapsed();
    let post_concurrent = RssKb::read();
    let c_throughput = combined.samples_us.len() as f64 / concurrent_elapsed.as_secs_f64();
    println!(
        "    elapsed: {:.2}s   throughput: {c_throughput:.1} msg/s",
        concurrent_elapsed.as_secs_f64()
    );
    println!(
        "    latency mean={:.2}ms  p50={:.2}ms  p95={:.2}ms  p99={:.2}ms  p99.9={:.2}ms",
        combined.mean_ms(),
        combined.percentile(0.50),
        combined.percentile(0.95),
        combined.percentile(0.99),
        combined.percentile(0.999),
    );
    println!(
        "    RSS pre: {:.1} MB   post: {:.1} MB   delta: {:+.1} MB",
        pre_concurrent.mb(),
        post_concurrent.mb(),
        post_concurrent.mb() - pre_concurrent.mb()
    );

    // --- Phase 4: final check ---
    let final_rss = RssKb::read();
    println!("\n[4] final RSS: {:.1} MB", final_rss.mb());
    println!(
        "    total delta from baseline: {:+.1} MB",
        final_rss.mb() - baseline.mb()
    );

    println!("\n=== done ===");
}

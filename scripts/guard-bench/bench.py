"""Benchmark Bielik-Guard vs HerBERT-PL-Guard on Polish samples.

Run:
  uv run --with transformers --with torch --with tabulate \
      scripts/guard-bench/bench.py [--threads N]
"""
import argparse, os, sys, time, statistics, pathlib
from typing import Dict, List, Tuple

HERE = pathlib.Path(__file__).parent
sys.path.insert(0, str(HERE))
from samples import SAMPLES  # noqa: E402

BIELIK = os.path.expanduser("~/models/bielik-guard")
HERBERT = os.path.expanduser("~/models/herbert-guard")

# MLCommons taxonomy used by HerBERT-PL-Guard
HERBERT_TAXONOMY = {
    "S1": "violent_crimes", "S2": "non_violent_crimes", "S3": "sex_related",
    "S4": "csam", "S5": "defamation", "S6": "specialized_advice",
    "S7": "privacy", "S8": "ip", "S9": "weapons", "S10": "hate",
    "S11": "self_harm", "S12": "sexual_content", "S13": "elections",
    "S14": "code_abuse", "safe": "safe",
}


def bench_model(name: str, path: str, threads: int) -> Dict:
    import torch
    from transformers import AutoTokenizer, AutoModelForSequenceClassification
    torch.set_num_threads(threads)
    print(f"\n=== {name} (threads={threads}) ===", flush=True)
    t0 = time.perf_counter()
    tok = AutoTokenizer.from_pretrained(path)
    mdl = AutoModelForSequenceClassification.from_pretrained(path).eval()
    load_ms = (time.perf_counter() - t0) * 1000
    print(f"  load: {load_ms:.0f} ms", flush=True)

    id2label = mdl.config.id2label
    is_multilabel = mdl.config.problem_type == "multi_label_classification"

    # Warmup
    with torch.no_grad():
        for _ in range(3):
            enc = tok("rozgrzewka", return_tensors="pt", truncation=True, max_length=256)
            mdl(**enc)

    # Per-sample latency (seq=realistic, single batch)
    single_times: List[float] = []
    predictions: List[Tuple[str, str, str, float, Dict[str, float]]] = []
    with torch.no_grad():
        for expected, text in SAMPLES:
            enc = tok(text, return_tensors="pt", truncation=True, max_length=256)
            t = time.perf_counter()
            out = mdl(**enc).logits[0]
            single_times.append((time.perf_counter() - t) * 1000)
            if is_multilabel:
                probs = torch.sigmoid(out).tolist()
                scores = {id2label[i]: probs[i] for i in range(len(probs))}
                # top label by score
                top_label = max(scores, key=scores.get)
                top_score = scores[top_label]
                any_flag = any(v > 0.5 for v in probs)
                pred_summary = f"{top_label}={top_score:.2f}" + (" [flag]" if any_flag else "")
            else:
                probs = torch.softmax(out, dim=-1).tolist()
                scores = {id2label[i]: probs[i] for i in range(len(probs))}
                top_label = max(scores, key=scores.get)
                top_score = scores[top_label]
                pred_summary = f"{top_label}={top_score:.2f}"
            predictions.append((expected, text, top_label, top_score, scores))

    # Batched throughput
    texts = [t for _, t in SAMPLES]
    batch_times: List[float] = []
    with torch.no_grad():
        for bs in (1, 4, 16, 32):
            enc = tok(texts[:bs], return_tensors="pt", truncation=True, padding=True, max_length=256)
            # warm
            mdl(**enc)
            t = time.perf_counter()
            mdl(**enc)
            elapsed = (time.perf_counter() - t) * 1000
            batch_times.append((bs, elapsed, elapsed / bs))

    return {
        "name": name,
        "load_ms": load_ms,
        "single_p50": statistics.median(single_times),
        "single_p95": sorted(single_times)[int(len(single_times) * 0.95)],
        "single_mean": statistics.mean(single_times),
        "batch_times": batch_times,
        "predictions": predictions,
        "is_multilabel": is_multilabel,
    }


def print_predictions(result: Dict):
    print(f"\n--- predictions ({result['name']}) ---")
    for expected, text, top, score, _ in result["predictions"]:
        snippet = text if len(text) < 70 else text[:67] + "..."
        print(f"  [{expected:10}] -> {top:12} {score:.2f}  | {snippet}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--threads", type=int, default=2,
                    help="CPU threads (default 2 to simulate small VPS)")
    args = ap.parse_args()

    results = []
    for name, path in [("Bielik-Guard", BIELIK), ("HerBERT-PL-Guard", HERBERT)]:
        results.append(bench_model(name, path, args.threads))

    print("\n=== LATENCY SUMMARY ===")
    print(f"{'model':20} {'load_ms':>8} {'p50':>7} {'p95':>7} {'mean':>7}   batched (bs -> ms/msg)")
    for r in results:
        batch_str = "  ".join(f"{bs}->{per:.1f}" for bs, _, per in r["batch_times"])
        print(f"{r['name']:20} {r['load_ms']:>8.0f} {r['single_p50']:>6.1f}ms {r['single_p95']:>6.1f}ms "
              f"{r['single_mean']:>6.1f}ms   {batch_str}")

    for r in results:
        print_predictions(r)

    # Cross-compare: agreement on "safe vs flagged"
    print("\n=== AGREEMENT (safe vs flagged) ===")
    b_preds = results[0]["predictions"]
    h_preds = results[1]["predictions"]
    agree = 0
    disagree = []
    for (exp, text, b_top, b_sc, b_sc_all), (_, _, h_top, h_sc, h_sc_all) in zip(b_preds, h_preds):
        # Bielik: flagged if any category > 0.5
        b_flag = any(v > 0.5 for v in b_sc_all.values())
        # HerBERT: flagged if top != safe with score > 0.5
        h_flag = (h_top != "safe" and h_sc > 0.5)
        if b_flag == h_flag:
            agree += 1
        else:
            disagree.append((exp, text, b_flag, h_flag, b_top, b_sc, h_top, h_sc))
    total = len(b_preds)
    print(f"  agreement: {agree}/{total} ({100*agree/total:.0f}%)")
    if disagree:
        print("  disagreements:")
        for exp, text, bf, hf, bt, bs, ht, hs in disagree:
            snippet = text if len(text) < 60 else text[:57] + "..."
            print(f"    [{exp:10}] B={'FLAG' if bf else 'ok  '}({bt}={bs:.2f})  "
                  f"H={'FLAG' if hf else 'ok  '}({ht}={hs:.2f}) | {snippet}")


if __name__ == "__main__":
    main()

"""Compare int8 ONNX predictions against fp32 PyTorch baseline.

uv run --with 'optimum[onnxruntime]' --with transformers --with torch \
    --python 3.12 scripts/guard-bench/verify_int8.py
"""
import os, sys, pathlib, time, statistics

HERE = pathlib.Path(__file__).parent
sys.path.insert(0, str(HERE))
from samples import SAMPLES  # noqa

SRC = os.path.expanduser("~/models/bielik-guard")
ONNX = os.path.expanduser("~/models/bielik-guard-onnx")


def main():
    import torch, numpy as np
    from transformers import AutoTokenizer, AutoModelForSequenceClassification
    from optimum.onnxruntime import ORTModelForSequenceClassification
    torch.set_num_threads(2)

    tok = AutoTokenizer.from_pretrained(SRC)
    pt = AutoModelForSequenceClassification.from_pretrained(SRC).eval()
    ort_fp32 = ORTModelForSequenceClassification.from_pretrained(ONNX, file_name="model.onnx")
    ort_int8 = ORTModelForSequenceClassification.from_pretrained(ONNX, file_name="model_quantized.onnx")

    labels = [pt.config.id2label[i] for i in range(len(pt.config.id2label))]
    print(f"labels: {labels}\n")

    max_diff_fp32 = 0.0
    max_diff_int8 = 0.0
    int8_latency = []
    fp32_onnx_latency = []
    pt_latency = []

    flip_count = 0
    print(f"{'expected':12} {'pt':14} {'ort_fp32':14} {'ort_int8':14}  text")
    for expected, text in SAMPLES:
        enc = tok(text, return_tensors="pt", truncation=True, max_length=256)

        t = time.perf_counter()
        with torch.no_grad():
            pt_logits = pt(**enc).logits[0]
        pt_latency.append((time.perf_counter() - t) * 1000)
        pt_probs = torch.sigmoid(pt_logits).numpy()

        t = time.perf_counter()
        of_logits = ort_fp32(**enc).logits[0]
        fp32_onnx_latency.append((time.perf_counter() - t) * 1000)
        of_probs = torch.sigmoid(of_logits).numpy()

        t = time.perf_counter()
        oi_logits = ort_int8(**enc).logits[0]
        int8_latency.append((time.perf_counter() - t) * 1000)
        oi_probs = torch.sigmoid(oi_logits).numpy()

        max_diff_fp32 = max(max_diff_fp32, float(np.abs(pt_probs - of_probs).max()))
        max_diff_int8 = max(max_diff_int8, float(np.abs(pt_probs - oi_probs).max()))

        pt_flag = {labels[i] for i, v in enumerate(pt_probs) if v > 0.5}
        oi_flag = {labels[i] for i, v in enumerate(oi_probs) if v > 0.5}
        if pt_flag != oi_flag:
            flip_count += 1
        pt_top = labels[int(pt_probs.argmax())]
        of_top = labels[int(of_probs.argmax())]
        oi_top = labels[int(oi_probs.argmax())]
        snippet = text if len(text) < 55 else text[:52] + "..."
        print(f"{expected:12} {pt_top:8}{pt_probs.max():.2f} {of_top:8}{of_probs.max():.2f} {oi_top:8}{oi_probs.max():.2f}  {snippet}")

    print(f"\nmax |Δprob| fp32-ONNX vs PyTorch: {max_diff_fp32:.4f}")
    print(f"max |Δprob| int8-ONNX vs PyTorch: {max_diff_int8:.4f}")
    print(f"threshold-0.5 flag set changes (int8 vs PyTorch): {flip_count}/{len(SAMPLES)}")
    print()
    print(f"latency p50 (2-thread): pt={statistics.median(pt_latency):.1f}ms  "
          f"ort_fp32={statistics.median(fp32_onnx_latency):.1f}ms  "
          f"ort_int8={statistics.median(int8_latency):.1f}ms")
    print(f"latency p95:            pt={sorted(pt_latency)[int(.95*len(pt_latency))]:.1f}ms  "
          f"ort_fp32={sorted(fp32_onnx_latency)[int(.95*len(fp32_onnx_latency))]:.1f}ms  "
          f"ort_int8={sorted(int8_latency)[int(.95*len(int8_latency))]:.1f}ms")


if __name__ == "__main__":
    main()

"""Export Bielik-Guard to ONNX (fp32) and dynamic int8 quantized ONNX.

Run:
  uv run --with 'optimum[onnxruntime]>=1.23' --with transformers --with torch \
      --python 3.12 scripts/guard-bench/export_onnx.py
"""
import os, pathlib, time, shutil

SRC = pathlib.Path(os.path.expanduser("~/models/bielik-guard"))
OUT = pathlib.Path(os.path.expanduser("~/models/bielik-guard-onnx"))


def main():
    from optimum.onnxruntime import ORTModelForSequenceClassification
    from optimum.onnxruntime.configuration import AutoQuantizationConfig
    from optimum.onnxruntime import ORTQuantizer
    from transformers import AutoTokenizer

    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)

    print(f"exporting {SRC} -> {OUT}")
    t = time.perf_counter()
    model = ORTModelForSequenceClassification.from_pretrained(str(SRC), export=True)
    model.save_pretrained(str(OUT))
    tok = AutoTokenizer.from_pretrained(str(SRC))
    tok.save_pretrained(str(OUT))
    print(f"  fp32 export done in {time.perf_counter()-t:.1f}s")
    fp32_size = sum(f.stat().st_size for f in OUT.glob("*.onnx"))
    print(f"  fp32 onnx size: {fp32_size/1e6:.1f} MB")

    # Dynamic int8 quantization (avx2 target — works on any modern x86)
    print("quantizing to int8 (avx2)...")
    t = time.perf_counter()
    quantizer = ORTQuantizer.from_pretrained(str(OUT))
    qconfig = AutoQuantizationConfig.avx2(is_static=False, per_channel=False)
    quantizer.quantize(save_dir=str(OUT), quantization_config=qconfig)
    print(f"  quant done in {time.perf_counter()-t:.1f}s")
    for f in sorted(OUT.glob("*.onnx")):
        print(f"  {f.name}: {f.stat().st_size/1e6:.1f} MB")


if __name__ == "__main__":
    main()

"""Export Marqo nsfw-image-detection-384 to ONNX (fp32) for the Rust
image-moderation engine.

Run:
  uv run --with 'timm>=1.0' --with 'onnx>=1.16' --with torch \
      --python 3.12 scripts/marqo-export/export_onnx.py

Produces ~/models/marqo-nsfw-onnx/model.onnx (~20 MB fp32). The Rust
loader expects the file at exactly that name; input tensor `input`
shape [1, 3, 384, 384] float32, output tensor `output` shape [1, 2].
"""
import os
import pathlib
import time

import timm
import torch

OUT = pathlib.Path(os.path.expanduser("~/models/marqo-nsfw-onnx"))
MODEL_ID = "hf_hub:Marqo/nsfw-image-detection-384"


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    out_path = OUT / "model.onnx"

    print(f"loading {MODEL_ID}")
    t = time.perf_counter()
    model = timm.create_model(MODEL_ID, pretrained=True).eval()
    print(f"  loaded in {time.perf_counter() - t:.1f}s")

    dummy = torch.zeros(1, 3, 384, 384, dtype=torch.float32)
    print(f"exporting -> {out_path}")
    t = time.perf_counter()
    torch.onnx.export(
        model,
        dummy,
        str(out_path),
        input_names=["input"],
        output_names=["output"],
        opset_version=17,
        dynamic_axes=None,
    )
    size_mb = out_path.stat().st_size / 1e6
    print(f"  done in {time.perf_counter() - t:.1f}s ({size_mb:.1f} MB)")

    cfg = model.pretrained_cfg
    print(f"label order: {cfg.get('label_names')}")


if __name__ == "__main__":
    main()

"""Generate fixture images and Python timm reference NSFW scores for the
Rust image-moderation E2E test.

Writes deterministic synthetic fixtures (a uniform colour patch, a
gradient, and a seeded random-noise tile) plus a `reference.json` that
the Rust integration test (`backend/tests/image_moderation.rs`) compares
its own inference against. Synthetic inputs are intentional — we don't
want the repo to ship real NSFW imagery, and the goal is preprocessing
parity, not classifier accuracy.

Run:
  uv run --with timm --with torch --with pillow --with numpy \\
      --python 3.12 scripts/marqo-export/run_reference.py \\
      [--out /tmp/nsfw-e2e/fixtures]
"""
import argparse
import json
import pathlib

import numpy as np
import timm
import torch
from PIL import Image
from timm.data import create_transform, resolve_model_data_config

MODEL_ID = "hf_hub:Marqo/nsfw-image-detection-384"


def synth(name: str, kind: str, out_dir: pathlib.Path) -> pathlib.Path:
    rng = np.random.default_rng(hash(name) & 0xFFFFFFFF)
    if kind == "noise":
        arr = rng.integers(0, 256, size=(800, 600, 3), dtype=np.uint8)
    elif kind == "gradient":
        x = np.linspace(0, 255, 600, dtype=np.uint8)
        y = np.linspace(0, 255, 800, dtype=np.uint8)
        r = np.tile(x, (800, 1))
        g = np.tile(y[:, None], (1, 600))
        b = ((r.astype(int) + g.astype(int)) // 2).astype(np.uint8)
        arr = np.stack([r, g, b], axis=-1)
    elif kind == "solid":
        arr = np.full((800, 600, 3), 200, dtype=np.uint8)
    else:
        raise ValueError(f"unknown kind: {kind}")
    p = out_dir / f"{name}.png"
    Image.fromarray(arr).save(p)
    return p


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="/tmp/nsfw-e2e/fixtures")
    args = ap.parse_args()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    print(f"loading {MODEL_ID}")
    model = timm.create_model(MODEL_ID, pretrained=True).eval()
    cfg = resolve_model_data_config(model)
    print(f"data cfg: {cfg}")
    print(f"label_names: {model.pretrained_cfg.get('label_names')}")
    transform = create_transform(**cfg, is_training=False)

    results: dict = {}
    for name, kind in [("noise", "noise"), ("gradient", "gradient"), ("solid", "solid")]:
        p = synth(name, kind, out)
        img = Image.open(p).convert("RGB")
        x = transform(img).unsqueeze(0)
        with torch.no_grad():
            logits = model(x).cpu().numpy()[0]
        probs = np.exp(logits - logits.max())
        probs = probs / probs.sum()
        nsfw = float(probs[0])
        print(f"{name}: logits={logits} nsfw={nsfw:.6f}")
        results[name] = {"path": str(p), "nsfw": nsfw, "logits": logits.tolist()}

    (out / "reference.json").write_text(json.dumps(results, indent=2))
    print(f"wrote {out / 'reference.json'}")


if __name__ == "__main__":
    main()

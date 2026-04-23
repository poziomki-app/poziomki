# Moderation model — deployment step

The API and worker containers read the int8-quantized Bielik-Guard model
from `/app/moderation`, mounted read-only from
`infra/ops/moderation/bielik-guard-onnx/` on the host.

**Before `docker compose up` on a fresh host**, place the model directory
there. It must contain at minimum:

- `model_quantized.onnx`
- `tokenizer.json`
- `config.json`
- `special_tokens_map.json`
- `tokenizer_config.json`

## How to produce the directory

The model is gated on Hugging Face (one-time license click per account):
<https://huggingface.co/speakleash/Bielik-Guard-0.1B-v1.1>.

Once access is granted, regenerate the quantized dir with:

```bash
export HF_TOKEN=<your hf token>
hf download speakleash/Bielik-Guard-0.1B-v1.1 --local-dir /tmp/bielik-raw
uv run --with 'optimum[onnxruntime]>=1.23' --with 'transformers>=4.53' \
       --with 'torch' --python 3.12 scripts/guard-bench/export_onnx.py
# output lands in ~/models/bielik-guard-onnx/
rsync -a ~/models/bielik-guard-onnx/ infra/ops/moderation/bielik-guard-onnx/
```

`model.onnx` (fp32, ~500 MB) is not required at runtime — only the
quantized file is used. Ship just the int8 artifacts.

## What happens if the model is missing?

There are two modes, selected by the `MODERATION_REQUIRED` env var.

**Strict mode — `MODERATION_REQUIRED=true`** (the default in
`docker-compose.prod.yml`). If `MODERATION_MODEL_PATH` is unset / empty
or the directory doesn't contain `model_quantized.onnx` /
`tokenizer.json`, boot fails. The container crash-loops until the model
is mounted. This is the safe default for production — a silent
unmoderated rollout is worse than an obvious outage.

**Lenient mode — `MODERATION_REQUIRED` unset / falsy.** Missing model =
service boots normally, logs a `warn`, and all handlers treat content
as clean. Intended for dev and staging where you reasonably want to
iterate without the gated artefact.

Other failures during model load — corrupt ONNX, malformed tokenizer,
ORT init errors — are always fatal regardless of mode. Those indicate
real bugs rather than a missing artefact.

Whichever mode you're in, two signals let ops alert on the engine
state:

- the `moderation_engine_loaded` Prometheus gauge (1 when the engine
  loaded successfully, 0 when disabled);
- the `moderation_verdicts_total` counter (expected to increment under
  any chat traffic when the engine is up).

## Container sizing (validated on OVH-class VPS)

Loaded engine peaks at ~370 MB RSS during warmup, stable at ~365 MB after
the first few inferences. `docker-compose.prod.yml` is sized accordingly:

- `api`: 768 MB memory / 1.0 CPU
- `worker`: 512 MB memory / 0.5 CPU

Validated via podman cgroup v2 enforcement of the `moderation-stress`
bench (`scripts/guard-bench/Dockerfile.bench`). Hard OOM reproducible at
≤ 320 MB; safe minimum is 384 MB. See the bench binary for re-running.

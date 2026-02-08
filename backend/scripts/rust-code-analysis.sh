#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RCA_BIN="${RUST_CODE_ANALYSIS_BIN:-rust-code-analysis-cli}"

if ! command -v "$RCA_BIN" >/dev/null 2>&1; then
  if [ -x "$HOME/.cargo/bin/rust-code-analysis-cli" ]; then
    RCA_BIN="$HOME/.cargo/bin/rust-code-analysis-cli"
  else
    echo "rust-code-analysis-cli not found. Install with: cargo install --locked rust-code-analysis-cli" >&2
    exit 127
  fi
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for rust-code-analysis quality checks." >&2
  exit 127
fi

MIN_MI="${RCA_MIN_MI_VISUAL_STUDIO:-8}"
MAX_CYCLOMATIC="${RCA_MAX_CYCLOMATIC:-8}"
MAX_COGNITIVE="${RCA_MAX_COGNITIVE:-6}"
MAX_EXITS="${RCA_MAX_EXITS:-3}"
MAX_NARGS="${RCA_MAX_NARGS:-4}"

TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

"$RCA_BIN" \
  --paths "$ROOT_DIR/src" \
  --paths "$ROOT_DIR/migration/src" \
  --metrics \
  --output-format json \
  --pr \
  --include '*.rs' \
  --exclude '**/target/**' \
  > "$TMP_FILE"

VIOLATIONS="$(
  jq -r -s \
    --argjson min_mi "$MIN_MI" \
    --argjson max_cyc "$MAX_CYCLOMATIC" \
    --argjson max_cog "$MAX_COGNITIVE" \
    --argjson max_exits "$MAX_EXITS" \
    --argjson max_nargs "$MAX_NARGS" \
    '
    map(
      select(.kind == "unit")
      | {
          name,
          mi: (.metrics.mi.mi_visual_studio // 0),
          cyclomatic: (.metrics.cyclomatic.max // 0),
          cognitive: (.metrics.cognitive.max // 0),
          exits: (.metrics.nexits.max // 0),
          nargs: (.metrics.nargs.functions_max // 0)
        }
      | select(
          (.mi < $min_mi)
          or (.cyclomatic > $max_cyc)
          or (.cognitive > $max_cog)
          or (.exits > $max_exits)
          or (.nargs > $max_nargs)
        )
      | "\(.name)\tmi_vs=\(.mi)\tcyclomatic=\(.cyclomatic)\tcognitive=\(.cognitive)\texits=\(.exits)\tnargs=\(.nargs)"
    )
    | .[]
    ' "$TMP_FILE"
)"

if [ -n "$VIOLATIONS" ]; then
  echo "rust-code-analysis quality gate failed." >&2
  echo "Thresholds: min_mi_vs=$MIN_MI max_cyclomatic=$MAX_CYCLOMATIC max_cognitive=$MAX_COGNITIVE max_exits=$MAX_EXITS max_nargs=$MAX_NARGS" >&2
  echo "$VIOLATIONS" >&2
  exit 1
fi

SUMMARY="$(
  jq -r -s '
    {
      files: length,
      min_mi_vs: (map(.metrics.mi.mi_visual_studio // 0) | min),
      max_cyclomatic: (map(.metrics.cyclomatic.max // 0) | max),
      max_cognitive: (map(.metrics.cognitive.max // 0) | max),
      max_exits: (map(.metrics.nexits.max // 0) | max),
      max_nargs: (map(.metrics.nargs.functions_max // 0) | max)
    }
  ' "$TMP_FILE"
)"

echo "rust-code-analysis quality gate passed."
echo "$SUMMARY"

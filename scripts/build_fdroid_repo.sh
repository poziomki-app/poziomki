#!/usr/bin/env bash
set -euo pipefail

if ! command -v fdroid >/dev/null 2>&1; then
  echo "fdroid command not found. Install fdroidserver first." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APK_DIR="${1:-}"
OUTPUT_DIR="${2:-$ROOT_DIR/dist/fdroid-repo}"

if [[ -z "$APK_DIR" || ! -d "$APK_DIR" ]]; then
  echo "Usage: $0 <apk-dir> [output-dir]" >&2
  exit 1
fi

: "${FDROID_REPO_URL:=https://poziomki.app/fdroid/repo}"
: "${FDROID_REPO_NAME:=Poziomki}"
: "${FDROID_REPO_DESCRIPTION:=Poziomki Android repository}"
: "${FDROID_KEYSTORE_FILE:?Set FDROID_KEYSTORE_FILE}"
: "${FDROID_KEYSTORE_PASSWORD:?Set FDROID_KEYSTORE_PASSWORD}"
: "${FDROID_KEY_ALIAS:?Set FDROID_KEY_ALIAS}"
: "${FDROID_KEY_PASSWORD:?Set FDROID_KEY_PASSWORD}"

WORK_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

mkdir -p "$WORK_DIR/repo" "$WORK_DIR/metadata"
cp "$APK_DIR"/*.apk "$WORK_DIR/repo/"
cp "$ROOT_DIR/fdroid/metadata/"*.yml "$WORK_DIR/metadata/"

cat > "$WORK_DIR/config.yml" <<EOF
repo_url: ${FDROID_REPO_URL}
repo_name: ${FDROID_REPO_NAME}
repo_description: ${FDROID_REPO_DESCRIPTION}
repo_icon: icon.png
keystore: ${FDROID_KEYSTORE_FILE}
keystorepass: ${FDROID_KEYSTORE_PASSWORD}
keypass: ${FDROID_KEY_PASSWORD}
repo_keyalias: ${FDROID_KEY_ALIAS}
archive_older: 0
EOF

python3 - <<'PY' "$WORK_DIR/repo/icon.png"
import base64
import pathlib
png = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAQAAAAAYLlVAAAAVUlEQVR4Ae3PQQ0AIBDAsAP/nsEEj4ZkVbCTdXfOTtQA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD8mADe1gABvV9WiwAAAABJRU5ErkJggg=="
)
pathlib.Path(__import__("sys").argv[1]).write_bytes(png)
PY

(
  cd "$WORK_DIR"
  fdroid update --create-metadata --verbose
)

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
cp -R "$WORK_DIR"/repo "$OUTPUT_DIR"/repo

printf 'Built self-hosted F-Droid repo bundle in %s\n' "$OUTPUT_DIR"

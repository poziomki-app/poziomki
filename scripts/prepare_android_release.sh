#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_FILE="$ROOT_DIR/mobile/androidApp/build.gradle.kts"
OUTPUT_DIR="${1:-$ROOT_DIR/dist/android-release}"

mkdir -p "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR"/*.apk "$OUTPUT_DIR"/*.txt "$OUTPUT_DIR"/*.tar.gz

version_name="$(sed -n 's/.*versionName = "\(.*\)"/\1/p' "$BUILD_FILE" | head -n1)"
version_code="$(sed -n 's/.*versionCode = \([0-9][0-9]*\).*/\1/p' "$BUILD_FILE" | head -n1)"

if [[ -z "$version_name" || -z "$version_code" ]]; then
  echo "Could not read versionName/versionCode from $BUILD_FILE" >&2
  exit 1
fi

apk_dir="$ROOT_DIR/mobile/androidApp/build/outputs/apk/release"
shopt -s nullglob
apks=("$apk_dir"/*.apk)

if [[ ${#apks[@]} -eq 0 ]]; then
  echo "No release APKs found in $apk_dir" >&2
  exit 1
fi

for apk in "${apks[@]}"; do
  name="$(basename "$apk")"
  target=""
  case "$name" in
    *universal*)
      target="poziomki-${version_name}-universal.apk"
      ;;
    *arm64-v8a*)
      target="poziomki-${version_name}-arm64-v8a.apk"
      ;;
    *armeabi-v7a*)
      target="poziomki-${version_name}-armeabi-v7a.apk"
      ;;
    *x86_64*)
      target="poziomki-${version_name}-x86_64.apk"
      ;;
  esac

  if [[ -n "$target" ]]; then
    cp "$apk" "$OUTPUT_DIR/$target"
  fi
done

(
  cd "$OUTPUT_DIR"
  sha256sum ./*.apk > "poziomki-${version_name}-sha256.txt"
)

tar -C "$ROOT_DIR" -czf "$OUTPUT_DIR/poziomki-${version_name}-metadata.tar.gz" \
  .fdroid.yml \
  docs/android-distribution.txt \
  docs/site \
  fastlane \
  fdroid

printf 'Prepared Android release assets for version %s (%s) in %s\n' "$version_name" "$version_code" "$OUTPUT_DIR"

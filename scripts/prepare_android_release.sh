#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_FILE="$ROOT_DIR/mobile/androidApp/build.gradle.kts"
OUTPUT_DIR="${1:-$ROOT_DIR/dist/android-release}"

mkdir -p "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR"/*.apk "$OUTPUT_DIR"/*.txt "$OUTPUT_DIR"/*.tar.gz

# Read appVersionName (source of truth, bumped by release-please) and derive
# versionCode from it using the same formula as build.gradle.kts.
version_name="$(sed -n 's/.*appVersionName = "\(.*\)".*/\1/p' "$BUILD_FILE" | head -n1)"

if [[ -z "$version_name" ]]; then
  echo "Could not read appVersionName from $BUILD_FILE" >&2
  exit 1
fi

IFS=. read -r v_major v_minor v_patch <<<"$version_name"
if [[ -z "$v_major" || -z "$v_minor" || -z "$v_patch" ]]; then
  echo "appVersionName '$version_name' is not major.minor.patch" >&2
  exit 1
fi
version_code=$(( v_major * 1000000 + v_minor * 1000 + v_patch ))

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
  fastlane \
  fdroid

printf 'Prepared Android release assets for version %s (%s) in %s\n' "$version_name" "$version_code" "$OUTPUT_DIR"

#!/usr/bin/env bash
# Run all Maestro flows on every poziomki AVD, sequentially.
# Artifacts (screenshots, logs) go to .maestro/artifacts/<avd>/.
set -uo pipefail

ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
ADB="$ANDROID_HOME/platform-tools/adb"
EMULATOR="$ANDROID_HOME/emulator/emulator"
MAESTRO="${MAESTRO:-$HOME/.maestro/bin/maestro}"

APK="mobile/androidApp/build/outputs/apk/debug/androidApp-x86_64-debug.apk"
FLOWS=".maestro/flows"
ARTIFACTS=".maestro/artifacts"
AVDS=("${@:-poziomki poziomki_small poziomki_tablet}")
# shellcheck disable=SC2206
AVDS=( $AVDS )

[[ -f "$APK" ]] || { echo "Missing $APK — run: gradle :androidApp:assembleDebug"; exit 1; }
command -v "$MAESTRO" >/dev/null || { echo "Maestro not found at $MAESTRO"; exit 1; }

mkdir -p "$ARTIFACTS"
overall_rc=0

for avd in "${AVDS[@]}"; do
  echo "=== AVD: $avd ==="
  out="$ARTIFACTS/$avd"
  mkdir -p "$out"

  # Boot
  "$EMULATOR" -avd "$avd" -no-window -no-snapshot-save -no-boot-anim \
    -gpu swiftshader_indirect -no-audio > "$out/emulator.log" 2>&1 &
  emu_pid=$!

  # Wait for boot
  for _ in $(seq 1 120); do
    state=$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    [[ "$state" == "1" ]] && break
    sleep 2
  done
  if [[ "$state" != "1" ]]; then
    echo "  ! $avd never booted; skipping"
    kill -9 "$emu_pid" 2>/dev/null
    overall_rc=1
    continue
  fi

  "$ADB" install -r "$APK" > "$out/install.log" 2>&1

  if "$MAESTRO" test "$FLOWS" --format junit --output "$out/report.xml" > "$out/maestro.log" 2>&1; then
    echo "  ✓ $avd passed"
  else
    echo "  ✗ $avd failed — see $out/maestro.log"
    overall_rc=1
  fi

  # Pull most recent Maestro screenshots
  latest=$(ls -dt "$HOME/.maestro/tests"/*/ 2>/dev/null | head -1)
  [[ -n "$latest" ]] && cp -r "$latest" "$out/screenshots" 2>/dev/null

  # Stop emulator
  "$ADB" -s emulator-5554 emu kill > /dev/null 2>&1
  wait "$emu_pid" 2>/dev/null
done

echo
echo "Done. Artifacts under $ARTIFACTS/."
exit $overall_rc

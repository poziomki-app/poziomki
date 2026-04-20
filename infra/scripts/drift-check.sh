#!/usr/bin/env bash
# Fail (non-zero exit) if the working tree has drifted from origin/main.
# Run nightly on the VPS via cron; alert to ntfy on failure.
set -euo pipefail

cd "$(dirname "$0")/.."

git fetch --quiet origin main

if ! git diff --quiet HEAD origin/main; then
  echo "drift: local HEAD differs from origin/main"
  git --no-pager diff --stat HEAD origin/main
  exit 1
fi

if ! git diff --quiet; then
  echo "drift: uncommitted changes to tracked files"
  git --no-pager diff --stat
  exit 2
fi

echo "clean"

#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
task_tmp="$(mktemp -d "${TMPDIR:-/tmp}/thumble-optical.XXXXXX")"
trap 'rm -rf "$task_tmp"' EXIT
CLANG_MODULE_CACHE_PATH="$task_tmp/clang" xcrun swiftc \
  -module-cache-path "$task_tmp/swift" \
  "$root/Sources/Bridge/WoWLiveCapture.swift" \
  "$root/scripts/fixtures/WoWOpticalPanelTests.swift" -o "$task_tmp/verify"
"$task_tmp/verify" "$root/Host/fixtures/wow-live/v1.json"

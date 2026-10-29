#!/usr/bin/env bash
# Runs a scripted check in a simulator (read-only on the phone: never used
# there). The scenario is page JavaScript, run through Demo.swift
# (KEEL_GLASS_DEMO="js:…") 4 s after launch, with diagnostics on; it reports
# with `check(msg)`, which writes "[check] msg" lines to the log. Screenshots
# are taken at the given seconds after launch.
#
#   scripts/sim-check.sh <udid> scripts/sim-checks/<name>.js [name@secs ...]
#
# Output: diag-logs/<stamp>-<name>/ with the PNGs and the pulled log.
set -euo pipefail
cd "$(dirname "$0")/.."
udid=$1; js=$2; shift 2
name=$(basename "$js" .js)
out="diag-logs/$(date +%Y%m%d-%H%M%S)-$name"
mkdir -p "$out"
if grep -q '|' "$js"; then echo "$js: no '|' allowed (Demo.swift splits on it)" >&2; exit 1; fi
prelude="const check = (m) => window.__TAURI__.core.invoke('diag_log', { source: 'check', msg: String(m) }); const sleep = (ms) => new Promise((r) => setTimeout(r, ms));"
code="(async () => { $prelude
$(cat "$js")
})().catch((e) => check('FAILED ' + e));"
xcrun simctl terminate "$udid" com.nshoe.keel 2>/dev/null || true
data=$(xcrun simctl get_app_container "$udid" com.nshoe.keel data)
rm -f "$data/Library/Caches/diag.log" "$data/Library/Caches/diag.1.log"
SIMCTL_CHILD_KEEL_DIAG=1 SIMCTL_CHILD_KEEL_GLASS_DEMO="js:$code" xcrun simctl launch "$udid" com.nshoe.keel >/dev/null
start=$(date +%s)
for shot in "$@"; do
  label=${shot%@*}; at=${shot#*@}
  while [ $(( $(date +%s) - start )) -lt "$at" ]; do sleep 0.2; done
  xcrun simctl io "$udid" screenshot "$out/$label.png" >/dev/null 2>&1
  echo "$out/$label.png"
done
cp "$data/Library/Caches/diag.log" "$out/diag.log" 2>/dev/null || true
grep '\[check\]' "$out/diag.log" 2>/dev/null || echo "(no check lines)"

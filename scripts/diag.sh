#!/usr/bin/env bash
# Pulls Keel's diagnostics log (Library/Caches/diag.log, plus diag.1.log if
# it has rolled over) from the iPhone or a simulator into diag-logs/,
# timestamped. Turn logging on first: Settings > Diagnostics logging, or
# launch with KEEL_DIAG=1 (simulator: SIMCTL_CHILD_KEEL_DIAG=1).
#
#   scripts/diag.sh                  # from the phone
#   scripts/diag.sh <simulator-udid> # from a simulator
#   scripts/diag.sh --fresh [udid]   # clear the device's log first, then exit
set -euo pipefail

PHONE=00008160-00082D8921400036
BUNDLE=com.nshoe.keel
OUT="$(cd "$(dirname "$0")/.." && pwd)/diag-logs"

fresh=0
udid=""
for a in "$@"; do
  case "$a" in
    --fresh) fresh=1 ;;
    -h|--help) sed -n '2,9p' "$0"; exit 0 ;;
    *) udid="$a" ;;
  esac
done

mkdir -p "$OUT"
stamp="$(date +%Y%m%d-%H%M%S)"

if [ -n "$udid" ]; then
  data="$(xcrun simctl get_app_container "$udid" "$BUNDLE" data)"
  dir="$data/Library/Caches"
  if [ "$fresh" = 1 ]; then
    rm -f "$dir/diag.log" "$dir/diag.1.log"
    echo "Cleared the simulator's log."
    exit 0
  fi
  for f in diag.1.log diag.log; do
    [ -f "$dir/$f" ] && cp "$dir/$f" "$OUT/$stamp-sim-$f"
  done
  src="sim"
else
  if [ "$fresh" = 1 ]; then
    # devicectl can't delete files; replace the log with an empty one.
    : > "$OUT/.empty"
    for f in diag.log diag.1.log; do
      xcrun devicectl device copy to --device "$PHONE" --domain-type appDataContainer \
        --domain-identifier "$BUNDLE" --source "$OUT/.empty" --destination "Library/Caches/$f" \
        --remove-existing-content true >/dev/null || true
    done
    rm -f "$OUT/.empty"
    echo "Cleared the phone's log."
    exit 0
  fi
  for f in diag.1.log diag.log; do
    xcrun devicectl device copy from --device "$PHONE" --domain-type appDataContainer \
      --domain-identifier "$BUNDLE" --source "Library/Caches/$f" \
      --destination "$OUT/$stamp-phone-$f" >/dev/null 2>&1 || rm -f "$OUT/$stamp-phone-$f"
  done
  src="phone"
fi

shopt -s nullglob
files=("$OUT/$stamp-$src-"*)
if [ ${#files[@]} -eq 0 ]; then
  echo "No diag.log found. Is logging on? (Settings > Diagnostics logging, or KEEL_DIAG=1)" >&2
  exit 1
fi
for f in "${files[@]}"; do
  echo "$f ($(wc -l < "$f" | tr -d ' ') lines)"
done

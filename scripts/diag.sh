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
    # Never write into the phone's app container from here: `devicectl device
    # copy to --remove-existing-content` empties the whole container (it wiped
    # keel.db on 2026-10-28). On the phone, switching Diagnostics logging on
    # in Settings starts a fresh log (the last one becomes diag.1.log).
    echo "--fresh only works for a simulator. On the phone, switch Diagnostics logging off, then on, in Settings: that starts a fresh log." >&2
    exit 1
  fi
  for f in diag.1.log diag.log; do
    xcrun devicectl device copy from --device "$PHONE" --domain-type appDataContainer \
      --domain-identifier "$BUNDLE" --source "Library/Caches/$f" \
      --destination "$OUT/$stamp-phone-$f" >/dev/null 2>&1 || rm -f "$OUT/$stamp-phone-$f"
    [ -s "$OUT/$stamp-phone-$f" ] || rm -f "$OUT/$stamp-phone-$f"
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

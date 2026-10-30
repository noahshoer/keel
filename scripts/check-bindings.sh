#!/usr/bin/env bash
# Regenerates the ts-rs IPC bindings (web/src/bindings, index.ts and
# commands.json included; the command list and …Args types come from keel)
# and fails when they differ from what is committed: a Rust type changed but
# the bindings weren't regenerated. Old files are removed first, so a binding
# whose Rust type is gone shows up as a deletion.
set -euo pipefail
cd "$(dirname "$0")/.."

dir=web/src/bindings
find "$dir" -maxdepth 1 -name '*.ts' -delete 2>/dev/null || true
# A failed cargo run would leave the bindings deleted: put back HEAD's.
restore() { git checkout HEAD -- "$dir"; echo "error: cargo failed; restored $dir from HEAD" >&2; }
trap restore ERR
# keel-core's first: Commands.ts imports its types.
cargo test -p keel-core --features ts --test ts_bindings --quiet
cargo test -p keel --features ts --test commands_list --quiet
trap - ERR

# Against HEAD, not the index: staged-but-uncommitted bindings still fail.
if ! git diff --exit-code --stat HEAD -- "$dir" || [ -n "$(git ls-files --others --exclude-standard -- "$dir")" ]; then
  git ls-files --others --exclude-standard -- "$dir" | sed 's/^/untracked: /'
  echo "error: $dir is stale; run scripts/check-bindings.sh and commit the result" >&2
  exit 1
fi
echo "bindings up to date"

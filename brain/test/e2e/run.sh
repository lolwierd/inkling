#!/bin/bash
# End-to-end run of Inkling against the fake brain, in the background, with a
# fresh notebook folder. Prints DATA=<folder> and EXIT=<code>; inspect with
#   python3 show.py <folder> [log|brain|notes]     python3 dumps.py <folder> [keys]
#
# usage: run.sh <scenario.txt> [timeout-seconds] [fixture-dir]
#   INKLING_BIN   app binary (default: ../../../target/debug/inkling)
#   DATA_ROOT     where notebook folders go (default: $TMPDIR/inkling-e2e)
#   REAL=1        use the real brain (model calls on the Claude login) instead of the fake
# Any FAKE_* variable (see fake-brain.ts) passes through, as does INKLING_JIGGLE.
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
script=$1; limit=${2:-40}; fixture=$3
bin=${INKLING_BIN:-$repo/target/debug/inkling}
root=${DATA_ROOT:-${TMPDIR:-/tmp}/inkling-e2e}
dir=$root/$(basename "$script" .txt)-$(date +%Y%m%d-%H%M%S)-$$
mkdir -p "$dir/notes"
if [ -n "$fixture" ] && [ -d "$fixture" ]; then cp -R "$fixture"/. "$dir"/; fi
echo "DATA=$dir"
if [ "$REAL" = 1 ]; then entry=(env -u INKLING_BRAIN_ENTRY); else entry=(env INKLING_BRAIN_ENTRY="$here/fake-brain.ts"); fi
"${entry[@]}" INKLING_BACKGROUND=1 INKLING_DATA_DIR="$dir" INKLING_SCRIPT="$script" RUST_LOG=${RUST_LOG:-inkling=info} \
  "$bin" > "$dir/app.log" 2>&1 &
pid=$!
for _ in $(seq 1 $((limit * 10))); do kill -0 $pid 2>/dev/null || break; sleep 0.1; done
if kill -0 $pid 2>/dev/null; then echo "TIMEOUT: stopping $pid"; kill $pid; sleep 0.5; kill -9 $pid 2>/dev/null; fi
wait $pid 2>/dev/null; echo "EXIT=$?"

#!/usr/bin/env bash
# Run one firmware on the micro:bit v2 and keep its defmt log.
# Usage: board-run.sh <example-dir> <bin> <run-number> [features]
#   example-dir   examples/microbit-v2-tflm or examples/microbit-v2-kws
#   BOARD_RUN_LIMIT_SECONDS   time limit, default 1500
# The log goes to <example-dir>/results/<bin>-run<N>.log, under a header that
# names the date, commit, tools and build. The script fails when the log lacks
# the two "cycles:" lines or the two passing result lines. A passing line is
# "within one step: true" for timing_int8 and "equal: true" for the others.
set -euo pipefail

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
  echo "usage: board-run.sh <example-dir> <bin> <run-number> [features]" >&2
  exit 2
fi
example_dir="$(cd "$1" && pwd)"
bin="$2"
run_number="$3"
features="${4:-}"
limit="${BOARD_RUN_LIMIT_SECONDS:-1500}"

# shellcheck source=board-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/board-lib.sh"

if [ "$bin" = "timing_int8" ]; then
  pass_text="within one step: true"
  fail_text="within one step: false"
else
  pass_text="equal: true"
  fail_text="equal: false"
fi

results_dir="$example_dir/results"
mkdir -p "$results_dir"
log="$results_dir/${bin}-run${run_number}.log"
candidate="$log.invalid"
if [ -e "$log" ] || [ -e "$candidate" ]; then
  echo "ERROR: $log or $candidate exists; move it first" >&2
  exit 1
fi
scratch_dir="$(mktemp -d)"
raw="$scratch_dir/raw.log"

elf="$(build_elf "$example_dir" "$bin" "$features")"
log_header "$example_dir" "$bin" "$features" > "$candidate"

run_finished() {
  [ "$(awk '/microseconds at/ { n++ } END { print n + 0 }' "$1")" -ge 2 ] || grep -q -E "ERROR|panick" "$1"
}
if ! run_on_board "$elf" "$raw" "$limit" run_finished; then
  echo "ERROR: time limit of $limit s ended before the run finished; see $raw" >&2
  filter_log "$raw" >> "$candidate"
  exit 1
fi

filter_log "$raw" >> "$candidate"

cycle_lines="$(awk '/cycles: min/ { n++ } END { print n + 0 }' "$candidate")"
pass_lines="$(awk -v text="$pass_text" 'index($0, text) { n++ } END { print n + 0 }' "$candidate")"
if [ "$cycle_lines" -ne 2 ] || [ "$pass_lines" -ne 2 ] || grep -q -E "$fail_text|ERROR|panick" "$candidate"; then
  echo "ERROR: $candidate is not valid: $cycle_lines cycle lines, $pass_lines lines with '$pass_text'" >&2
  exit 1
fi
mv "$candidate" "$log"
echo "OK: $log"
echo "raw probe-rs output: $raw"

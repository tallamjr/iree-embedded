#!/usr/bin/env bash
# Find the smallest arena size that gives correct logits, by bisection on the
# board. Each step builds the firmware with the arena size in an environment
# variable, flashes it, and reads the result over defmt RTT.
# Usage: arena-bisect.sh <example-dir> <bin> <env-var> <fail-bytes> <pass-bytes> [features]
#   fail-bytes   an arena size that must fail (checked first)
#   pass-bytes   an arena size that must pass (checked first)
# Examples:
#   arena-bisect.sh examples/microbit-v2-tflm arena_tflm TFLM_ARENA_BYTES 4096 16384
#   arena-bisect.sh examples/microbit-v2-kws arena_int8 IREE_ARENA_BYTES 16384 57344 int8
# The log goes to <example-dir>/results/arena-bisect-<bin>.log.
set -euo pipefail

if [ "$#" -lt 5 ] || [ "$#" -gt 6 ]; then
  echo "usage: arena-bisect.sh <example-dir> <bin> <env-var> <fail-bytes> <pass-bytes> [features]" >&2
  exit 2
fi
example_dir="$(cd "$1" && pwd)"
bin="$2"
env_var="$3"
low="$4"   # always fails
high="$5"  # always passes
features="${6:-}"

# shellcheck source=board-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/board-lib.sh"

results_dir="$example_dir/results"
mkdir -p "$results_dir"
log="$results_dir/arena-bisect-${bin}.log"
if [ -e "$log" ]; then
  echo "ERROR: $log exists; move it first" >&2
  exit 1
fi
scratch_dir="$(mktemp -d)"

{
  echo "# date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# commit: $(git -C "$example_dir" rev-parse HEAD)"
  echo "# binary: $bin, features: ${features:-none}, variable: $env_var"
  echo "# arm-none-eabi-g++: $(arm-none-eabi-g++ --version | sed -n 1p)"
  echo "# rustc: $(rustc --version)"
} > "$log"

result_seen() {
  grep -q -E "equal: |within one step: |ERROR|panick" "$1"
}

# Prints "pass" or "fail". Appends the log lines of the step to the log file.
try_arena() {
  local bytes="$1"
  export "$env_var=$bytes"
  local elf raw="$scratch_dir/raw-$bytes.log"
  elf="$(build_elf "$example_dir" "$bin" "$features")"
  if ! run_on_board "$elf" "$raw" 120 result_seen; then
    echo "ERROR: no result for arena $bytes within 120 s; see $raw" >&2
    return 1
  fi
  {
    echo "## arena $bytes"
    filter_log "$raw"
  } >> "$log"
  if grep -q -E "equal: true|within one step: true" "$raw" && ! grep -q -E "ERROR|panick" "$raw"; then
    echo pass
  else
    echo fail
  fi
}

if [ "$(try_arena "$low")" != "fail" ]; then
  echo "ERROR: arena $low did not fail; pick a smaller fail-bytes" >&2
  exit 1
fi
if [ "$(try_arena "$high")" != "pass" ]; then
  echo "ERROR: arena $high did not pass; pick a larger pass-bytes" >&2
  exit 1
fi
while [ $((high - low)) -gt 1 ]; do
  middle=$(((low + high) / 2))
  outcome="$(try_arena "$middle")"
  echo "arena $middle: $outcome"
  if [ "$outcome" = "pass" ]; then
    high="$middle"
  else
    low="$middle"
  fi
done
{
  echo "## result"
  echo "smallest passing arena: $high bytes ($low bytes fails)"
} | tee -a "$log"

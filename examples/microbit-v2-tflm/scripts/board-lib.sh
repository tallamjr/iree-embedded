#!/usr/bin/env bash
# Shared functions for board-run.sh and arena-bisect.sh. Source it; do not run it.
# Needs cargo, jq, probe-rs and grep on PATH.

readonly BOARD_CHIP="nRF52833_xxAA"

# Build one binary in an example folder and print the path of its ELF file.
# Usage: build_elf <example-dir> <bin> [features]
build_elf() {
  local example_dir="$1" bin="$2" features="${3:-}"
  local feature_args=()
  if [ -n "$features" ]; then
    feature_args=(--features "$features")
  fi
  local elf
  elf="$(cd "$example_dir" && cargo build --release "${feature_args[@]}" --bin "$bin" \
    --message-format=json-render-diagnostics \
    | jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable')"
  if [ -z "$elf" ] || [ ! -f "$elf" ]; then
    echo "ERROR: cargo built no ELF file for $bin" >&2
    return 1
  fi
  printf '%s\n' "$elf"
}

# Flash an ELF file and write the probe-rs output to a file. probe-rs never
# exits by itself, so stop it when the output shows that the run is finished or the time ends.
# Returns 1 when the time ends first.
# Usage: run_on_board <elf> <raw-file> <limit-seconds> <done-function>
# The function gets the raw file and returns 0 when the run is finished.
run_on_board() {
  local elf="$1" raw="$2" limit="$3" is_done="$4"
  probe-rs run --chip "$BOARD_CHIP" "$elf" > "$raw" 2>&1 &
  local probe_pid=$!
  local waited=0 finished=1
  while kill -0 "$probe_pid" 2> /dev/null; do
    if "$is_done" "$raw"; then
      finished=0
      break
    fi
    if [ "$waited" -ge "$limit" ]; then
      break
    fi
    sleep 2
    waited=$((waited + 2))
  done
  if kill -0 "$probe_pid" 2> /dev/null; then
    kill "$probe_pid"
  fi
  # probe-rs exits non-zero after a kill or a panic; the log check decides.
  if wait "$probe_pid"; then
    echo "probe-rs ended with status 0" >&2
  fi
  if [ "$finished" -ne 0 ] && "$is_done" "$raw"; then
    finished=0
  fi
  return "$finished"
}

# Keep the defmt lines of a raw probe-rs file.
# Usage: filter_log <raw-file>
filter_log() {
  # defmt colours its output; strip the colour codes so the log is plain text.
  local lines
  lines="$(perl -pe 's/\e\[[0-9;]*m//g' "$1" | grep -E "INFO|ERROR|WARN|panick|error")" || {
    echo "ERROR: no log lines in $1" >&2
    return 1
  }
  printf '%s\n' "$lines"
}

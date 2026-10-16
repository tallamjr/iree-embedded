#!/usr/bin/env bash
# Flash timing_usb to the Nano 33 BLE Sense, read its USB serial output and
# keep a checked log. Usage: scripts/board-run.sh <run-number>
#   BOARD_RUN_LIMIT_SECONDS   time limit from the end of flashing, default 900
# Before you start, put the board in bootloader mode: double-tap reset until
# the orange LED fades in and out. The timed run takes about four minutes, and
# the port appears only after it. The log goes to results/timing_usb-run<N>.log
# under a header with the date, commit, tools, build and serial port. The
# script runs validate-log.sh on it. A log that fails stays as a .invalid file.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: board-run.sh <run-number>" >&2
  exit 2
fi
run_number="$1"
limit="${BOARD_RUN_LIMIT_SECONDS:-900}"
bin="timing_usb"
features="usb-serial"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
example_dir="$(cd "$here/.." && pwd)"
results_dir="$example_dir/results"
mkdir -p "$results_dir"
log="$results_dir/${bin}-run${run_number}.log"
candidate="$log.invalid"
if [ -e "$log" ] || [ -e "$candidate" ]; then
  echo "ERROR: $log or $candidate exists; move it first" >&2
  exit 1
fi

for tool in cargo jq arm-none-eabi-gcc stty; do
  if ! command -v "$tool" > /dev/null; then
    echo "ERROR: $tool not found on PATH" >&2
    exit 1
  fi
done

# Print the first serial port, or nothing. macOS names ports
# /dev/cu.usbmodem*; Linux names them /dev/ttyACM*.
find_port() {
  local pattern port
  if [ "$(uname -s)" = "Darwin" ]; then
    pattern="/dev/cu.usbmodem*"
  else
    pattern="/dev/ttyACM*"
  fi
  for port in $pattern; do
    if [ -e "$port" ]; then
      printf '%s\n' "$port"
      return 0
    fi
  done
  return 0
}

# Make a port raw so that line endings and echo do not change the text.
set_raw() {
  if [ "$(uname -s)" = "Darwin" ]; then
    /bin/stty -f "$1" raw -echo
  else
    stty -F "$1" raw -echo
  fi
}

# True when the file holds a full result or an error line.
read_finished() {
  [ "$(tr -d '\r' < "$1" | awk '/^microseconds at/ { n++ } END { print n + 0 }')" -ge 2 ] ||
    grep -q -E "ERROR|panick" "$1"
}

# Print one result from a raw file: from the first header line to the second
# microseconds line, or to the end of the file when the result is cut short.
extract_result() {
  tr -d '\r' < "$1" | awk '
    /^iree-embedded KWS timing: / { started = 1 }
    /ERROR/ { started = 1 }
    started { print }
    started && /^microseconds at/ && ++count == 2 { exit }
  '
}

elf="$(cd "$example_dir" && cargo build --release --features "$features" --bin "$bin" \
  --message-format=json-render-diagnostics |
  jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable')"
if [ -z "$elf" ] || [ ! -f "$elf" ]; then
  echo "ERROR: cargo built no ELF file for $bin" >&2
  exit 1
fi

"$here/flash.sh" "$elf"

# The board resets after flashing. The bootloader port goes away, and the
# firmware port appears when the timed run ends.
sleep 5
deadline=$((SECONDS + limit))
port=""
while [ -z "$port" ]; do
  if [ "$SECONDS" -ge "$deadline" ]; then
    echo "ERROR: no serial port appeared within $limit s" >&2
    exit 1
  fi
  sleep 2
  port="$(find_port)"
done
echo "== serial port $port appeared; reading =="

raw_dir="$(mktemp -d)"
raw="$raw_dir/raw.log"
set_raw "$port"
cat "$port" > "$raw" &
reader_pid=$!
finished=1
while kill -0 "$reader_pid" 2> /dev/null; do
  if read_finished "$raw"; then
    finished=0
    break
  fi
  if [ "$SECONDS" -ge "$deadline" ]; then
    break
  fi
  sleep 2
done
if kill -0 "$reader_pid" 2> /dev/null; then
  kill "$reader_pid"
fi
# cat ends with a signal after the kill, so its status says nothing.
if wait "$reader_pid"; then
  echo "serial reader ended with status 0" >&2
fi

tree_state="clean"
if [ -n "$(git -C "$example_dir" status --porcelain -- . ':!results')" ]; then
  tree_state="dirty"
fi
repo_root="$(git -C "$example_dir" rev-parse --show-toplevel)"
if [ -n "${IREE_RUNTIME_DIR:-}${IREE_SRC_DIR:-}" ]; then
  runtime_source="environment variables"
elif [ -d "$repo_root/.iree/build/mcu" ] && [ -d "$repo_root/.iree/src" ]; then
  runtime_source="in-repo .iree folder"
else
  runtime_source="prebuilt download"
fi
{
  echo "# date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# commit: $(git -C "$example_dir" rev-parse HEAD) (tree $tree_state)"
  echo "# binary: $bin"
  echo "# features: $features"
  echo "# arm-none-eabi-gcc: $(arm-none-eabi-gcc --version | sed -n 1p)"
  echo "# rustc: $(rustc --version)"
  echo "# board: Arduino Nano 33 BLE Sense, nRF52840, 64 MHz, USB serial"
  echo "# serial port: $port"
  echo "# iree-embedded-sys: $(sed -n 's/^version = "\(.*\)"/\1/p' "$repo_root/crates/iree-embedded-sys/Cargo.toml" | sed -n 1p)"
  echo "# IREE_RUNTIME_DIR: ${IREE_RUNTIME_DIR:-unset}"
  echo "# IREE_SRC_DIR: ${IREE_SRC_DIR:-unset}"
  echo "# iree runtime source: $runtime_source"
  extract_result "$raw"
} > "$candidate"

if [ "$finished" -ne 0 ]; then
  echo "ERROR: time limit of $limit s ended before the result was complete; see $candidate" >&2
  exit 1
fi
if ! "$here/validate-log.sh" "$candidate"; then
  echo "ERROR: $candidate is not valid; raw serial output: $raw" >&2
  exit 1
fi
mv "$candidate" "$log"
echo "OK: $log"
echo "raw serial output: $raw"

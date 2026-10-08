#!/usr/bin/env bash
# Host test for the C++ shim. It builds the TFLite-Micro sources and the shim
# with the host compiler, runs the model on the fixed features and checks that
# the logits equal the reference exactly.
#
# Usage: scripts/host-check.sh   (run scripts/fetch-tflm.sh first)
# Objects go to target/host-check/. CXX sets the compiler (default: c++).
set -euo pipefail

EXAMPLE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TREE="$EXAMPLE/tflm-tree"
OUT="$EXAMPLE/target/host-check"
CXX_BIN="${CXX:-c++}"

if [ ! -d "$TREE" ]; then
  echo "error: $TREE not found; run scripts/fetch-tflm.sh first" >&2
  exit 1
fi
mkdir -p "$OUT"

flags=(-std=c++17 -O1 -DTF_LITE_STRIP_ERROR_STRINGS -DTF_LITE_DISABLE_X86_NEON
  -fno-rtti -fno-exceptions -Wall -Wextra -Wno-unused-parameter
  -I"$TREE" -I"$TREE/third_party/flatbuffers/include" -I"$EXAMPLE/src")

compile() {
  local source="$1" object="$2"
  if [ ! -f "$object" ] || [ "$source" -nt "$object" ]; then
    "$CXX_BIN" "${flags[@]}" -c "$source" -o "$object"
  fi
}

objects=()
sources=()
while IFS= read -r line; do
  case "$line" in '#'* | '') continue ;; esac
  sources+=("$TREE/$line")
done < "$EXAMPLE/scripts/tflm-sources.txt"
sources+=("$EXAMPLE/src/shim.cc" "$EXAMPLE/src/platform.cc" "$EXAMPLE/tests/host_check.cc")

pids=()
for source in "${sources[@]}"; do
  object="$OUT/$(printf '%s' "${source#"$EXAMPLE"/}" | tr '/' '_').o"
  objects+=("$object")
  compile "$source" "$object" &
  pids+=("$!")
  if [ "${#pids[@]}" -ge 8 ]; then
    wait "${pids[0]}"
    pids=("${pids[@]:1}")
  fi
done
for pid in "${pids[@]}"; do
  wait "$pid"
done

"$CXX_BIN" "${objects[@]}" -o "$OUT/host_check"
"$OUT/host_check" "$EXAMPLE/models/micro_speech_int8_nosm.tflite" "$EXAMPLE/models/yes_features_int8.bin"

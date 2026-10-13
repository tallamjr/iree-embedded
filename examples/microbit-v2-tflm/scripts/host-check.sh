#!/usr/bin/env bash
# Host test for the C++ shim. It builds the TFLite-Micro sources and the shim
# with the host compiler, runs the model on the fixed features and checks that
# the logits equal the reference exactly.
#
# Usage: scripts/host-check.sh [reference|cmsis-nn]   (default: reference)
# Run scripts/fetch-tflm.sh first. Objects go to target/host-check-<variant>/.
# CXX and CC set the compilers (default: c++ and cc). The cmsis-nn variant
# builds the CMSIS-NN C code without the Cortex-M SIMD instructions.
set -euo pipefail

EXAMPLE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VARIANT="${1:-reference}"
CXX_BIN="${CXX:-c++}"
CC_BIN="${CC:-cc}"
case "$VARIANT" in
  reference)
    TREE="$EXAMPLE/tflm-tree"
    KERNEL_LIST="$EXAMPLE/scripts/tflm-kernels-reference.txt"
    ;;
  cmsis-nn)
    TREE="$EXAMPLE/tflm-tree-cmsis-nn"
    KERNEL_LIST="$EXAMPLE/scripts/tflm-kernels-cmsis-nn.txt"
    ;;
  *)
    echo "usage: host-check.sh [reference|cmsis-nn]" >&2
    exit 2
    ;;
esac
OUT="$EXAMPLE/target/host-check-$VARIANT"

if [ ! -d "$TREE" ]; then
  echo "error: $TREE not found; run scripts/fetch-tflm.sh first" >&2
  exit 1
fi
mkdir -p "$OUT"

flags=(-O1 -DTF_LITE_STRIP_ERROR_STRINGS -DTF_LITE_DISABLE_X86_NEON
  -Wall -Wextra -Wno-unused-parameter
  -I"$TREE" -I"$TREE/third_party/flatbuffers/include" -I"$EXAMPLE/src")
if [ "$VARIANT" = cmsis-nn ]; then
  flags+=(-DCMSIS_NN -DARM_NN_ENABLE_F16=0 -DARM_NN_ENABLE_F32=0
    -I"$TREE/third_party/cmsis_nn" -I"$TREE/third_party/cmsis_nn/Include"
    -I"$TREE/third_party/cmsis" -I"$TREE/third_party/cmsis/CMSIS/Core/Include")
fi

compile() {
  local source="$1" object="$2"
  if [ ! -f "$object" ] || [ "$source" -nt "$object" ]; then
    case "$source" in
      *.c) "$CC_BIN" "${flags[@]}" -c "$source" -o "$object" ;;
      *) "$CXX_BIN" -std=c++17 -fno-rtti -fno-exceptions "${flags[@]}" -c "$source" -o "$object" ;;
    esac
  fi
}

add_list() {
  local list="$1" prefix="$2" line
  while IFS= read -r line; do
    case "$line" in '#'* | '') continue ;; esac
    sources+=("$prefix$line")
  done < "$list"
}

objects=()
sources=()
add_list "$EXAMPLE/scripts/tflm-sources.txt" "$TREE/"
add_list "$KERNEL_LIST" "$TREE/"
if [ "$VARIANT" = cmsis-nn ]; then
  add_list "$EXAMPLE/scripts/cmsis-nn-sources.txt" "$TREE/third_party/cmsis_nn/Source/"
fi
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

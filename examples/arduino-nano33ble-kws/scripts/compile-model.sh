#!/usr/bin/env bash
# Regenerate the model artefacts in models/ from their committed sources.
#
# Stage 2 (default): MLIR -> iree-compile -> models/*.{o,h,vmfb}. Needs only
# the pinned iree-compile (pip install "iree-base-compiler==$IREE_PIP_VERSION",
# see scripts/iree-version.env at the repo root). This is what CI runs.
#
# Stage 1 (--from-tflite): models/micro_speech.tflite -> tf2onnx -> ONNX
# (graph truncated at the pre-softmax tensor; the firmware argmaxes raw
# logits, so the softmax is dead weight) -> iree-import-onnx ->
# models/micro_speech_nosm.mlir. Needs the heavier conversion toolchain
# (see scripts/requirements-model.txt for the env recipe). Run occasionally
# (e.g. after bumping the IREE pin); never needed in CI. Note: tf2onnx's
# auto-generated resource names are not stable across runs, so a rerun
# differs cosmetically from the committed MLIR while compiling identically.
set -euo pipefail

EXAMPLE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$EXAMPLE/../.." && pwd)"
MODELS="$EXAMPLE/models"

# shellcheck source=/dev/null
source "$ROOT/scripts/iree-version.env"

COMPILE="${IREE_COMPILE:-$ROOT/.venv/bin/iree-compile}"
IMPORT="${IREE_IMPORT_ONNX:-$ROOT/.venv/bin/iree-import-onnx}"
PYTHON_MODEL="${PYTHON_MODEL:-$ROOT/.venv-model/bin/python}"

if [ ! -x "$COMPILE" ]; then
    echo "error: $COMPILE not found." >&2
    echo "install with: python3 -m venv $ROOT/.venv && $ROOT/.venv/bin/pip install \"iree-base-compiler==$IREE_PIP_VERSION\"" >&2
    exit 1
fi

# The flags the committed artefacts were built with: static-library kernels
# for the Cortex-M4F plus a 32-bit-index VM program.
MCU_FLAGS=(
    --iree-hal-target-device=local
    --iree-hal-local-target-device-backends=llvm-cpu
    --iree-llvmcpu-target-triple=thumbv7em-none-eabihf
    --iree-llvmcpu-target-cpu=cortex-m4
    --iree-llvmcpu-target-float-abi=hard
    --iree-llvmcpu-link-embedded=false
    --iree-llvmcpu-link-static
    --iree-vm-target-index-bits=32
)

# Accept no arguments or exactly --from-tflite; reject anything else so a typo
# does not silently fall through to a stage-2-only run.
if [ "$#" -gt 1 ] || { [ "$#" -eq 1 ] && [ "$1" != "--from-tflite" ]; }; then
    echo "usage: ${BASH_SOURCE[0]} [--from-tflite]" >&2
    exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ "${1:-}" = "--from-tflite" ]; then
    if [ ! -x "$PYTHON_MODEL" ]; then
        echo "error: $PYTHON_MODEL not found." >&2
        echo "install with: uv venv --python 3.11 $ROOT/.venv-model && uv pip install --python $ROOT/.venv-model -r $ROOT/scripts/requirements-model.txt" >&2
        exit 1
    fi
    if [ ! -x "$IMPORT" ]; then
        echo "error: $IMPORT not found." >&2
        echo "install with: python3 -m venv $ROOT/.venv && $ROOT/.venv/bin/pip install \"iree-base-compiler==$IREE_PIP_VERSION\"" >&2
        echo "the importer additionally needs the onnx package: $ROOT/.venv/bin/pip install onnx" >&2
        exit 1
    fi
    # add_1_dequant is the dequantised f32 input of the model's softmax (the
    # raw uint8 tensor is add_1); cutting there keeps the [1,4] f32 logits
    # signature the firmware expects. See the example README's provenance
    # section for how the name was found.
    "$PYTHON_MODEL" -m tf2onnx.convert \
        --tflite "$MODELS/micro_speech.tflite" \
        --opset 13 \
        --outputs 'add_1_dequant' \
        --output "$work/micro_speech_nosm.onnx"
    # Write through the temp dir and mv on success so a mid-importer failure
    # cannot truncate the committed MLIR.
    "$IMPORT" "$work/micro_speech_nosm.onnx" -o "$work/micro_speech_nosm.mlir"
    mv "$work/micro_speech_nosm.mlir" "$MODELS/micro_speech_nosm.mlir"
    echo "regenerated $MODELS/micro_speech_nosm.mlir"
fi

# micro_speech: compiled as micro_speech_nosm.o so the kernel query symbol
# stays micro_speech_nosm_linked_library_query (what the firmware links),
# then renamed to the committed micro_speech.{o,h} filenames.
(cd "$work" && "$COMPILE" "${MCU_FLAGS[@]}" \
    --iree-llvmcpu-static-library-output-path=micro_speech_nosm.o \
    "$MODELS/micro_speech_nosm.mlir" -o micro_speech.vmfb)
mv "$work/micro_speech_nosm.o" "$MODELS/micro_speech.o"
mv "$work/micro_speech_nosm.h" "$MODELS/micro_speech.h"
mv "$work/micro_speech.vmfb" "$MODELS/micro_speech.vmfb"

echo "regenerated MCU artefacts in $MODELS"

#!/usr/bin/env bash
# Regenerate the model artefacts in models/ from their committed sources.
#
# Stage 2 (default): MLIR -> iree-compile -> models/*.{o,h,vmfb}. Needs only
# the pinned iree-compile (pip install "iree-base-compiler==$IREE_PIP_VERSION",
# see scripts/iree-version.env at the repo root). This is what CI runs.
#
# Usage: compile-model.sh [--from-tflite] [uint8|int8]
# The model name defaults to uint8: micro_speech and simple_mul, as before.
# int8 builds only the upstream int8 micro_speech from models/int8/ and writes
# models/micro_speech_int8.{o,h,vmfb}, so the uint8 files stay untouched.
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

# Accept --from-tflite and a model name (uint8 or int8), each at most once;
# reject anything else so a typo does not silently fall through to a
# stage-2-only run.
FROM_TFLITE=0
MODEL_NAME=uint8
SEEN_MODEL=0
for arg in "$@"; do
    case "$arg" in
        --from-tflite) FROM_TFLITE=1 ;;
        uint8 | int8)
            if [ "$SEEN_MODEL" -eq 1 ]; then
                echo "usage: ${BASH_SOURCE[0]} [--from-tflite] [uint8|int8]" >&2
                exit 1
            fi
            SEEN_MODEL=1
            MODEL_NAME="$arg"
            ;;
        *)
            echo "usage: ${BASH_SOURCE[0]} [--from-tflite] [uint8|int8]" >&2
            exit 1
            ;;
    esac
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Per-model names. uint8 keeps the original names; int8 uses new ones that
# cannot clash with them.
if [ "$MODEL_NAME" = "int8" ]; then
    TFLITE_SOURCE="$MODELS/int8/micro_speech_quantized.tflite"
    MLIR_NAME=micro_speech_int8_nosm
    MLIR_PATH="$MODELS/int8/micro_speech_int8_nosm.mlir"
    OUTPUT_STEM=micro_speech_int8
    # The compiler embeds the input and output paths in its files. Relative
    # paths from models/ keep the committed int8 files the same in every
    # checkout.
    MLIR_ARG=int8/micro_speech_int8_nosm.mlir
    BUILD_DIR="$MODELS"
else
    TFLITE_SOURCE="$MODELS/micro_speech.tflite"
    MLIR_NAME=micro_speech_nosm
    MLIR_PATH="$MODELS/micro_speech_nosm.mlir"
    OUTPUT_STEM=micro_speech
    MLIR_ARG="$MLIR_PATH"
    BUILD_DIR="$work"
fi

if [ "$FROM_TFLITE" -eq 1 ]; then
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
    # raw uint8 or int8 tensor is add_1); cutting there keeps the [1,4] f32 logits
    # signature the firmware expects. See the example README's provenance
    # section for how the name was found.
    "$PYTHON_MODEL" -m tf2onnx.convert \
        --tflite "$TFLITE_SOURCE" \
        --opset 13 \
        --outputs 'add_1_dequant' \
        --output "$work/$MLIR_NAME.onnx"
    # Write through the temp dir and mv on success so a mid-importer failure
    # cannot truncate the committed MLIR.
    "$IMPORT" "$work/$MLIR_NAME.onnx" -o "$work/$MLIR_NAME.mlir"
    mv "$work/$MLIR_NAME.mlir" "$MLIR_PATH"
    echo "regenerated $MLIR_PATH"
fi

# micro_speech: compiled as $MLIR_NAME.o so the kernel query symbol stays
# ${MLIR_NAME}_linked_library_query (what the firmware links), then renamed to
# the committed $OUTPUT_STEM.{o,h} filenames.
(cd "$BUILD_DIR" && "$COMPILE" "${MCU_FLAGS[@]}" \
    --iree-llvmcpu-static-library-output-path="$MLIR_NAME.o" \
    "$MLIR_ARG" -o "$OUTPUT_STEM.vmfb")
mv "$BUILD_DIR/$MLIR_NAME.o" "$MODELS/$OUTPUT_STEM.o"
mv "$BUILD_DIR/$MLIR_NAME.h" "$MODELS/$OUTPUT_STEM.h"
if [ "$BUILD_DIR" != "$MODELS" ]; then
    mv "$BUILD_DIR/$OUTPUT_STEM.vmfb" "$MODELS/$OUTPUT_STEM.vmfb"
fi

if [ "$MODEL_NAME" = "int8" ]; then
    echo "regenerated int8 MCU artefacts in $MODELS"
    exit 0
fi

# simple_mul: single dispatch, so the query symbol is
# simple_mul_dispatch_0_library_query and no rename is needed.
(cd "$work" && "$COMPILE" "${MCU_FLAGS[@]}" \
    --iree-llvmcpu-static-library-output-path=simple_mul.o \
    "$MODELS/simple_mul.mlir" -o simple_mul.vmfb)
mv "$work"/simple_mul.{o,h,vmfb} "$MODELS/"

echo "regenerated MCU artefacts in $MODELS"

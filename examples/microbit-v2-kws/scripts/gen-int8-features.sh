#!/usr/bin/env bash
# Regenerate models/int8/yes_features_int8.bin from the pinned upstream tree.
#
# The file holds the 1,960 int8 feature bytes of the upstream "yes" clip. The
# upstream audio front end (audio_preprocessor_int8.tflite) makes them, run by
# TFLite-Micro on the host. Both runtimes in the int8 test read this one file.
#
# Usage: scripts/gen-int8-features.sh <tflite-micro-tree> [output-file]
# The tree must be a clone of tensorflow/tflite-micro at the pinned commit.
# On macOS the Homebrew `ar` makes archives that `ld` rejects, so the script
# puts /usr/bin first on PATH. The make files need GNU make 3.82 or later
# (set MAKE=gmake on macOS) and the Python package Pillow.
set -euo pipefail

readonly PINNED_COMMIT=5328e0d852c420f9494a53135529677229dfcdd2
readonly WAV_SHA256=b40e137a9d25bbdf135e03fa0f6bff4696d0bfc203aef8bc73b27c9428ba5a92

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "usage: ${BASH_SOURCE[0]} <tflite-micro-tree> [output-file]" >&2
    exit 1
fi

EXAMPLE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TREE="$(cd "$1" && pwd)"
OUTPUT="${2:-$EXAMPLE/models/int8/yes_features_int8.bin}"
MICRO_SPEECH="$TREE/tensorflow/lite/micro/examples/micro_speech"
PREPROCESSOR="$MICRO_SPEECH/models/audio_preprocessor_int8.tflite"
WAV="$MICRO_SPEECH/testdata/yes_1000ms.wav"

actual_commit="$(git -C "$TREE" rev-parse HEAD)"
if [ "$actual_commit" != "$PINNED_COMMIT" ]; then
    echo "error: tree is at $actual_commit, expected $PINNED_COMMIT" >&2
    exit 1
fi
actual_wav_sha="$(shasum -a 256 "$WAV" | cut -d' ' -f1)"
if [ "$actual_wav_sha" != "$WAV_SHA256" ]; then
    echo "error: $WAV has SHA-256 $actual_wav_sha, expected $WAV_SHA256" >&2
    exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# 16 kHz mono int16 samples without the WAV header.
python3 -I - "$WAV" "$work/yes.s16le" <<'PY'
import sys
import wave

with wave.open(sys.argv[1]) as wav_file:
    assert wav_file.getframerate() == 16000
    assert wav_file.getnchannels() == 1
    assert wav_file.getsampwidth() == 2
    frames = wav_file.readframes(wav_file.getnframes())
with open(sys.argv[2], "wb") as raw_file:
    raw_file.write(frames)
PY

PATH="/usr/bin:$PATH" "${MAKE:-make}" -C "$TREE" -f tensorflow/lite/micro/tools/make/Makefile -j8 microlite
library="$(find "$TREE/gen" -name libtensorflow-microlite.a \
    \( -path '*/osx_*_default_gcc/*' -o -path '*/linux_*_default_gcc/*' \))"
if [ "$(printf '%s\n' "$library" | wc -l)" -ne 1 ]; then
    echo "error: expected exactly one host library, found: $library" >&2
    exit 1
fi

downloads="$TREE/tensorflow/lite/micro/tools/make/downloads"
"${CXX:-c++}" -std=c++17 -O1 -DTF_LITE_STATIC_MEMORY \
    -I"$TREE" -I"$downloads/flatbuffers/include" -I"$downloads/kissfft" \
    "$EXAMPLE/scripts/tflm_features.cc" "$library" -o "$work/tflm_features"

"$work/tflm_features" "$PREPROCESSOR" "$work/yes.s16le" "$OUTPUT"
echo "wrote $OUTPUT"
shasum -a 256 "$OUTPUT"

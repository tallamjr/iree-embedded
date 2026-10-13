#!/usr/bin/env bash
# Fetch the pinned TFLite-Micro source and generate the two source trees that
# build.rs compiles. Run it once before `cargo build`.
#
# Usage: scripts/fetch-tflm.sh
# Output: tflm-src/ (shallow clone), tflm-tree/ (reference kernels) and
# tflm-tree-cmsis-nn/ (CMSIS-NN kernels, CMSIS-NN and CMSIS-Core sources). All
# three are git-ignored. TFLite-Micro's own download scripts fetch the CMSIS
# zips and check their MD5 sums. Needs network access, git, wget, unzip, GNU
# make 3.82 or later as `make`, Python 3 and arm-none-eabi-g++ on PATH. Set
# PYTHON to choose the interpreter. To make a tree again, delete its folder.
set -euo pipefail

readonly PINNED_COMMIT=5328e0d852c420f9494a53135529677229dfcdd2
readonly UPSTREAM_URL=https://github.com/tensorflow/tflite-micro.git
readonly UPSTREAM_MODEL_PATH=tensorflow/lite/micro/examples/micro_speech/models/micro_speech_quantized.tflite
readonly UPSTREAM_MODEL_SHA256=09e5e2a9dfb2d8ed78802bf18ce297bff54281a66ca18e0c23d69ca14f822a83
readonly LOCAL_MODEL_SHA256=34f10a8ea3f5b75919d2c65b4a2990179cc7c04962b77e0094fd69617cb9196a
# Revisions that the pinned TFLite-Micro downloads for the CMSIS-NN variant.
readonly CMSIS_NN_COMMIT=4ab83cc3cc98fb85ed6dafb55e8ca02f1628dcae
readonly CMSIS_CORE_COMMIT=5782d6f8057906d360f4b95ec08a2354afe5c9b9

EXAMPLE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE="$EXAMPLE/tflm-src"
TREE="$EXAMPLE/tflm-tree"
TREE_CMSIS="$EXAMPLE/tflm-tree-cmsis-nn"
PYTHON_BIN="${PYTHON:-python3}"

for tool in git make wget unzip shasum arm-none-eabi-g++ "$PYTHON_BIN"; do
  if ! command -v "$tool" > /dev/null; then
    echo "error: $tool not found on PATH" >&2
    exit 1
  fi
done
if ! make --version | head -n 1 | grep -q 'GNU Make \([4-9]\|3\.8[2-9]\)'; then
  echo "error: make must be GNU make 3.82 or later (on macOS: brew install make, then put its gnubin first on PATH)" >&2
  exit 1
fi
check_sha256() {
  local file="$1" expected="$2" actual
  actual="$(shasum -a 256 "$file" | cut -d' ' -f1)"
  if [ "$actual" != "$expected" ]; then
    echo "error: $file has SHA-256 $actual, expected $expected" >&2
    exit 1
  fi
}

if [ ! -d "$SOURCE/.git" ]; then
  git init --quiet "$SOURCE"
  git -C "$SOURCE" remote add origin "$UPSTREAM_URL"
  git -C "$SOURCE" fetch --quiet --depth 1 origin "$PINNED_COMMIT"
  git -C "$SOURCE" checkout --quiet --detach FETCH_HEAD
fi
actual_commit="$(git -C "$SOURCE" rev-parse HEAD)"
if [ "$actual_commit" != "$PINNED_COMMIT" ]; then
  echo "error: $SOURCE is at $actual_commit, expected $PINNED_COMMIT" >&2
  exit 1
fi

check_sha256 "$SOURCE/$UPSTREAM_MODEL_PATH" "$UPSTREAM_MODEL_SHA256"
check_sha256 "$EXAMPLE/models/micro_speech_int8_nosm.tflite" "$LOCAL_MODEL_SHA256"

# The download scripts of the pinned commit must use the revisions above.
make_dir="$SOURCE/tensorflow/lite/micro/tools/make/ext_libs"
grep -q "ZIP_PREFIX_NN=\"$CMSIS_NN_COMMIT\"" "$make_dir/cmsis_nn_download.sh" || {
  echo "error: cmsis_nn_download.sh does not pin CMSIS-NN $CMSIS_NN_COMMIT" >&2
  exit 1
}
grep -q "ZIP_PREFIX=\"$CMSIS_CORE_COMMIT\"" "$make_dir/cmsis_download.sh" || {
  echo "error: cmsis_download.sh does not pin CMSIS $CMSIS_CORE_COMMIT" >&2
  exit 1
}

# The generator copies the sources and headers that the Makefile lists for
# this target, plus the third-party files that the Makefile downloads.
toolchain_root="$(dirname "$(command -v arm-none-eabi-g++)")/"
generate_tree() {
  local tree="$1" extra_options="$2"
  if [ -e "$tree" ]; then
    echo "$tree exists; delete it to generate it again"
    return
  fi
  (
    cd "$SOURCE"
    "$PYTHON_BIN" -I tensorflow/lite/micro/tools/project_generation/create_tflm_tree.py \
      "$tree" \
      --makefile_options "TARGET=cortex_m_generic TARGET_ARCH=cortex-m4+fp TARGET_TOOLCHAIN_ROOT=$toolchain_root $extra_options"
  )
  echo "TFLite-Micro tree ready: $tree"
}
generate_tree "$TREE" ""
generate_tree "$TREE_CMSIS" "OPTIMIZED_KERNEL_DIR=cmsis_nn"

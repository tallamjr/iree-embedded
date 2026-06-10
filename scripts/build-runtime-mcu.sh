#!/usr/bin/env bash
# Cross-build the IREE runtime for a bare-metal Cortex-M target.
#
#   scripts/build-runtime-mcu.sh [cortex-m4f|cortex-m7|cortex-m33]
#
# Output: .iree/build/mcu for cortex-m4f (the default the sys crate links),
# .iree/build/<target> otherwise; override with IREE_BUILD. Host tools come
# from IREE_HOST_BIN_DIR (default .venv/bin, i.e. the pinned pip release).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-cortex-m4f}"
TOOLCHAIN="$ROOT/toolchains/arm-${TARGET}.cmake"
[ -f "$TOOLCHAIN" ] || { echo "unknown target '$TARGET' (no $TOOLCHAIN)"; exit 1; }

SRC="${IREE_SRC:-$ROOT/.iree/src}"
if [ "$TARGET" = "cortex-m4f" ]; then
  BUILD="${IREE_BUILD:-$ROOT/.iree/build/mcu}"
else
  BUILD="${IREE_BUILD:-$ROOT/.iree/build/$TARGET}"
fi
HOST_BIN="${IREE_HOST_BIN_DIR:-$ROOT/.venv/bin}"

"$ROOT/scripts/fetch-iree.sh"

echo "== cmake configure (cross: $TARGET) =="
rm -rf "$BUILD"
cmake -G Ninja -S "$SRC" -B "$BUILD" \
  --toolchain "$TOOLCHAIN" \
  -DIREE_HOST_BIN_DIR="$HOST_BIN" \
  -DCMAKE_POSITION_INDEPENDENT_CODE=OFF \
  -DIREE_ENABLE_POSITION_INDEPENDENT_CODE=OFF \
  -DIREE_BUILD_COMPILER=OFF \
  -DIREE_BUILD_TESTS=OFF \
  -DIREE_BUILD_SAMPLES=OFF \
  -DIREE_ENABLE_WERROR_FLAG=OFF \
  -DCMAKE_BUILD_TYPE=MinSizeRel

echo "== build runtime libs for the board =="
cmake --build "$BUILD" --target \
  iree_runtime_unified \
  iree_hal_drivers_local_sync_sync_driver \
  iree_hal_local_loaders_embedded_elf_loader \
  iree_hal_local_loaders_static_library_loader
echo "== DONE: $BUILD =="

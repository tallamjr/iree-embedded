#!/usr/bin/env bash
# Build the host IREE runtime that iree-embedded-sys links for `cargo test`.
# Output: .iree/build/host (override with IREE_BUILD).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${IREE_SRC:-$ROOT/.iree/src}"
BUILD="${IREE_BUILD:-$ROOT/.iree/build/host}"

"$ROOT/scripts/fetch-iree.sh"

if [[ "$(uname)" == "Darwin" ]]; then
  # Homebrew clang fails on macOS SDK mach headers; use Apple's toolchain.
  export CC="$(xcrun -f clang)"
  export CXX="$(xcrun -f clang++)"
  export SDKROOT="$(xcrun --show-sdk-path)"
fi

echo "== cmake configure (host) =="
rm -rf "$BUILD"
cmake -G Ninja -S "$SRC" -B "$BUILD" \
  ${CC:+-DCMAKE_C_COMPILER="$CC"} \
  ${CXX:+-DCMAKE_CXX_COMPILER="$CXX"} \
  -DIREE_BUILD_COMPILER=OFF \
  -DIREE_BUILD_TESTS=OFF \
  -DIREE_BUILD_SAMPLES=OFF \
  -DIREE_HAL_DRIVER_DEFAULTS=OFF \
  -DIREE_HAL_DRIVER_LOCAL_SYNC=ON \
  -DIREE_HAL_EXECUTABLE_LOADER_DEFAULTS=OFF \
  -DIREE_HAL_EXECUTABLE_LOADER_EMBEDDED_ELF=ON \
  -DIREE_HAL_EXECUTABLE_LOADER_STATIC_LIBRARY=ON \
  -DCMAKE_BUILD_TYPE=Release

echo "== build the libs we link =="
cmake --build "$BUILD" --target \
  iree_runtime_unified \
  iree_hal_drivers_local_sync_sync_driver \
  iree_hal_local_loaders_embedded_elf_loader \
  iree_hal_local_loaders_static_library_loader
echo "== DONE: $BUILD =="

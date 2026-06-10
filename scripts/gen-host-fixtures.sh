#!/usr/bin/env bash
# Regenerate the host-test fixtures in crates/iree-embedded/tests/fixtures.
#
# Host tests load models through the embedded-ELF loader, whose kernels are
# architecture-specific, so each model is compiled once per host arch and the
# tests select by cfg(target_arch). Uses the pinned iree-compile (see
# scripts/iree-version.env) and the IREE checkout for simple_mul.mlir.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${IREE_SRC:-$ROOT/.iree/src}"
FIX="$ROOT/crates/iree-embedded/tests/fixtures"
COMPILE="${IREE_COMPILE:-$ROOT/.venv/bin/iree-compile}"

for arch in x86_64 aarch64; do
  for model in simple_mul micro_speech; do
    case "$model" in
      simple_mul) mlir="$SRC/runtime/src/iree/runtime/demo/simple_mul.mlir" ;;
      micro_speech) mlir="$FIX/micro_speech_nosm.mlir" ;;
    esac
    "$COMPILE" \
      --iree-hal-target-device=local \
      --iree-hal-local-target-device-backends=llvm-cpu \
      --iree-llvmcpu-target-triple="${arch}-unknown-unknown-eabi-elf" \
      "$mlir" -o "$FIX/${model}-${arch}.vmfb"
    echo "wrote $FIX/${model}-${arch}.vmfb"
  done
done

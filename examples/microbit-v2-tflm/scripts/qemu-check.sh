#!/usr/bin/env bash
# Run the firmware code on an emulated Cortex-M4F (qemu mps2-an386), with no
# board. It builds qemu_tflm and runs it with semihosting output.
# Usage: scripts/qemu-check.sh [reference|cmsis-nn]   (default: reference)
# Needs qemu-system-arm and jq. The cmsis-nn variant uses a separate target
# directory, so the two ELF files do not overwrite each other.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
variant="${1:-reference}"
case "$variant" in
  reference) build_args=(--release) ;;
  cmsis-nn) build_args=(--release --features cmsis-nn --target-dir target/cmsis-nn) ;;
  *)
    echo "usage: qemu-check.sh [reference|cmsis-nn]" >&2
    exit 2
    ;;
esac
elf="$(cargo build "${build_args[@]}" --bin qemu_tflm --message-format=json-render-diagnostics \
  | jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable')"
timeout 120 qemu-system-arm -M mps2-an386 -cpu cortex-m4 -nographic \
  -semihosting-config enable=on,target=native -kernel "$elf"

#!/usr/bin/env bash
# Run the firmware code on an emulated Cortex-M4F (qemu mps2-an386), with no
# board. It builds qemu_tflm and runs it with semihosting output.
# Usage: scripts/qemu-check.sh   (needs qemu-system-arm and jq)
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
elf="$(cargo build --release --bin qemu_tflm --message-format=json-render-diagnostics \
  | jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable')"
timeout 120 qemu-system-arm -M mps2-an386 -cpu cortex-m4 -nographic \
  -semihosting-config enable=on,target=native -kernel "$elf"

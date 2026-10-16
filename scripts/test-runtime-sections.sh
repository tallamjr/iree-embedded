#!/usr/bin/env bash
# Check that a cross-built IREE runtime has one section per function.
# Without that, the linker cannot drop unused functions such as the
# bytecode disassembler, and it keeps the libc code they call.
#
#   scripts/test-runtime-sections.sh <build-dir>
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: test-runtime-sections.sh <build-dir>" >&2
  exit 2
fi
archive="$1/runtime/src/iree/runtime/libiree_runtime_unified.a"
if [ ! -f "$archive" ]; then
  echo "ERROR: $archive not found" >&2
  exit 1
fi
sections="$(arm-none-eabi-objdump -h "$archive")"
if ! grep -q '\.text\.iree_vm_bytecode_disassembler_emit_type_name' <<< "$sections"; then
  echo "FAIL: no per-function sections in $archive" >&2
  exit 1
fi
echo "OK: per-function sections in $archive"

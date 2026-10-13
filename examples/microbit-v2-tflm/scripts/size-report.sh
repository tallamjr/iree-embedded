#!/usr/bin/env bash
# Print a markdown flash and RAM report, and the symbol split, for the release
# firmware. The symbol split is the script of the IREE example, so that every
# firmware is grouped by the same rules.
# Usage: scripts/size-report.sh [elf]
# With no argument it builds timing_tflm (reference kernels) and
# timing_tflm_cmsis (CMSIS-NN kernels, with the cmsis-nn feature) and reports
# both. The two builds use separate target directories under target/.
set -euo pipefail

readonly FLASH_TOTAL_BYTES=524288 # nRF52833: 512 KB
readonly RAM_TOTAL_BYTES=131072   # nRF52833: 128 KB

SYMBOL_SPLIT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../../microbit-v2-kws/scripts/symbol-split.sh"

for tool in arm-none-eabi-size cargo jq awk; do
  if ! command -v "$tool" > /dev/null; then
    echo "ERROR: $tool not found on PATH" >&2
    exit 1
  fi
done

report() {
  local elf="$1"
  if [ ! -f "$elf" ]; then
    echo "ERROR: ELF not found: $elf" >&2
    exit 1
  fi
  echo "### $(basename "$elf")"
  echo
  # The vector table is flash content, so it counts in the .text row.
  arm-none-eabi-size -A "$elf" | awk \
    -v flash_total="$FLASH_TOTAL_BYTES" -v ram_total="$RAM_TOTAL_BYTES" '
    $1 == ".vector_table" || $1 == ".text" { text += $2 }
    $1 == ".rodata" { rodata = $2 }
    $1 == ".data" { data = $2 }
    $1 == ".uninit" { uninit = $2 }
    $1 == ".bss" { bss = $2 }
    END {
      flash = text + rodata + data
      ram = data + bss + uninit
      print "| Item | Bytes | Share of nRF52833 |"
      print "|---|---:|---:|"
      printf "| .text | %d | |\n", text
      printf "| .rodata | %d | |\n", rodata
      printf "| .data | %d | |\n", data
      printf "| .bss | %d | |\n", bss
      printf "| Flash total (.text + .rodata + .data) | %d | %.1f%% of 512 KB |\n", flash, 100 * flash / flash_total
      printf "| .uninit | %d | |\n", uninit
      printf "| Static RAM (.data + .bss + .uninit) | %d | %.1f%% of 128 KB |\n", ram, 100 * ram / ram_total
    }'
  echo
  "$SYMBOL_SPLIT" "$elf"
}

if [ "$#" -gt 1 ]; then
  echo "usage: size-report.sh [elf]" >&2
  exit 1
fi

if [ "$#" -eq 1 ]; then
  report "$1"
  exit 0
fi

cd "$(dirname "$0")/.."
build_elf() {
  local target_dir="$1"
  shift
  cargo build --release --target-dir "$target_dir" "$@" --message-format=json-render-diagnostics \
    | jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable'
}
reference_elf="$(build_elf target/reference --bin timing_tflm)"
cmsis_elf="$(build_elf target/cmsis-nn --features cmsis-nn --bin timing_tflm_cmsis)"
if [ -z "$reference_elf" ] || [ -z "$cmsis_elf" ]; then
  echo "ERROR: cargo built no binary" >&2
  exit 1
fi
report "$reference_elf"
report "$cmsis_elf"

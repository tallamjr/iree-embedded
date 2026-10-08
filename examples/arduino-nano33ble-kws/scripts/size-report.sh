#!/usr/bin/env bash
# Print a markdown flash and RAM report for the release firmware.
# Usage: scripts/size-report.sh [elf]
# With no argument it builds every release binary and reports each one.
set -euo pipefail

readonly FLASH_TOTAL_BYTES=1048576 # nRF52840: 1 MB
readonly RAM_TOTAL_BYTES=262144    # nRF52840: 256 KB

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
      print "| Item | Bytes | Share of nRF52840 |"
      print "|---|---:|---:|"
      printf "| .text | %d | |\n", text
      printf "| .rodata | %d | |\n", rodata
      printf "| .data | %d | |\n", data
      printf "| .bss | %d | |\n", bss
      printf "| Flash total (.text + .rodata + .data) | %d | %.1f%% of 1 MB |\n", flash, 100 * flash / flash_total
      printf "| .uninit | %d | |\n", uninit
      printf "| Static RAM (.data + .bss + .uninit) | %d | %.1f%% of 256 KB |\n", ram, 100 * ram / ram_total
    }'
  echo
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
# cargo prints one JSON line per built artefact; keep the binaries.
elfs="$(cargo build --release --bins --message-format=json-render-diagnostics \
  | jq -r 'select(.reason == "compiler-artifact" and .executable != null) | .executable')"
if [ -z "$elfs" ]; then
  echo "ERROR: cargo built no binaries" >&2
  exit 1
fi
while IFS= read -r elf; do
  report "$elf"
done <<< "$elfs"

#!/usr/bin/env bash
# Print flash and static RAM per symbol group for a release firmware ELF.
# The same rules work for the IREE firmware and for both TFLite-Micro
# firmware images, so the runtime shares can be compared.
#
# Usage: symbol-split.sh <elf> [--list <group>]
# --list prints the symbols of one group (pipe it to sort -rn to check the rules).
# Groups, first rule that matches the demangled symbol name wins:
#   model data     the model bytes in flash (IREE .vmfb, TFLM .tflite)
#   model kernels  code that runs the model operators (IREE compiled dispatch
#                  functions; TFLM depthwise, fully connected and reshape
#                  kernels with their Eval, Init and Prepare functions,
#                  reference_integer_ops and CMSIS-NN arm_*)
#   runtime        IREE (iree_*) or TFLite-Micro (tflite::, flatbuffers::,
#                  tflm_*) symbols that are not kernels
#   hal            embassy, nRF, cortex-m, defmt and panic symbols
#   rust           other Rust symbols (app, core, compiler_builtins)
#   libc           everything else (newlib, libgcc, vectors, C glue)
# Flash is the symbols in text, rodata and data. Static RAM is the symbols in
# data and bss. Aliases at one address count once.
set -euo pipefail

if [ "$#" -ne 1 ] && [ "$#" -ne 3 ]; then
  echo "usage: symbol-split.sh <elf> [--list <group>]" >&2
  exit 2
fi
elf="$1"
list_group=""
if [ "$#" -eq 3 ]; then
  if [ "$2" != "--list" ]; then
    echo "usage: symbol-split.sh <elf> [--list <group>]" >&2
    exit 2
  fi
  list_group="$3"
fi
for tool in arm-none-eabi-nm arm-none-eabi-size awk; do
  if ! command -v "$tool" > /dev/null; then
    echo "ERROR: $tool not found on PATH" >&2
    exit 1
  fi
done
if [ ! -f "$elf" ]; then
  echo "ERROR: ELF not found: $elf" >&2
  exit 1
fi

# Section totals from the linker, to show what the symbols do not cover.
size_totals="$(arm-none-eabi-size -A "$elf" | awk '
  $1 == ".vector_table" || $1 == ".text" || $1 == ".rodata" || $1 == ".data" { flash += $2 }
  $1 == ".data" || $1 == ".bss" || $1 == ".uninit" { ram += $2 }
  END { print flash, ram }')"

# Each nm line is: address size type name. Lines without a size are skipped.
arm-none-eabi-nm -S -C --defined-only "$elf" | awk \
  -v totals="$size_totals" -v list_group="$list_group" -v elf="$(basename "$elf")" '
function hex(text,   i, value) {
  value = 0
  text = tolower(text)
  for (i = 1; i <= length(text); i++) value = value * 16 + index("0123456789abcdef", substr(text, i, 1)) - 1
  return value
}
function group_of(name) {
  if (name ~ /^microbit_v2_kws::int8::VMFB/ || name ~ /^microbit_v2_tflm::MODEL/) return "model data"
  if (name ~ /^tf2onnx\$/ || name ~ /^micro_speech_int8_nosm_linked_library_query/ \
      || name ~ /^arm_/ || name ~ /reference_integer_ops::/ \
      || name ~ /^tflite::micro::\(anonymous namespace\)::(Eval|Init|Prepare)[A-Za-z0-9]*\(/ \
      || (name ~ /^tflite::micro::(\(anonymous namespace\)::)?[A-Za-z]*(DepthwiseConv|FullyConnected|Reshape)/ \
      && name !~ /^tflite::micro::Parse/)) return "model kernels"
  if (name ~ /^iree_/ || name ~ /^_?iree/ || name ~ /tflite::/ || name ~ /flatbuffers::/ \
      || name ~ /^tflm_/) return "runtime"
  if (name ~ /^<?(embassy_|nrf_pac|nrf|cortex_m|defmt|panic_probe|static_cell|critical_section|portable_atomic)/) return "hal"
  if (name ~ /::/ || name ~ /^</) return "rust"
  return "libc"
}
BEGIN { split(totals, t, " "); flash_total = t[1]; ram_total = t[2] }
NF >= 4 && length($2) == 8 && length($3) == 1 {
  address = $1; size = hex($2); type = $3
  name = $4
  for (i = 5; i <= NF; i++) name = name " " $i
  is_flash = (type ~ /[TtRrDdWwVv]/)
  is_ram = (type ~ /[DdBb]/)
  if (size == 0 || !(is_flash || is_ram) || (address in seen)) next
  seen[address] = 1
  group = group_of(name)
  if (list_group != "") {
    if (group == list_group) printf "%8d %s %s\n", size, type, name
    next
  }
  if (is_flash) flash[group] += size
  if (is_ram) ram[group] += size
}
END {
  if (list_group != "") exit
  n = split("runtime|model kernels|model data|hal|rust|libc", order, "|")
  print "### " elf " symbol split"
  print ""
  print "| Group | Flash bytes | Flash share | Static RAM bytes |"
  print "|---|---:|---:|---:|"
  for (i = 1; i <= n; i++) {
    g = order[i]
    printf "| %s | %d | %.1f%% | %d |\n", g, flash[g], 100 * flash[g] / flash_total, ram[g]
    flash_sum += flash[g]; ram_sum += ram[g]
  }
  printf "| not covered by symbols | %d | %.1f%% | %d |\n", flash_total - flash_sum, 100 * (flash_total - flash_sum) / flash_total, ram_total - ram_sum
  printf "| total (size -A) | %d | 100.0%% | %d |\n", flash_total, ram_total
  print ""
  printf "Runtime-only share of flash: %.1f%% (%d of %d bytes).\n", 100 * flash["runtime"] / flash_total, flash["runtime"], flash_total
  print ""
}'

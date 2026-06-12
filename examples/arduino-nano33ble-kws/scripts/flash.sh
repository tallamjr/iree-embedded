#!/usr/bin/env bash
# Cargo runner: flash the Nano 33 BLE Sense over USB via the stock Arduino
# bootloader (bossac). Usage (via cargo): scripts/flash.sh <path-to-elf>
#
# The bootloader occupies flash below 0x10000 and is never erased; the app
# (linked at 0x10000 by memory.x) is what bossac writes; the bootloader's
# SAM-BA emulation maps the writes to the app region itself, so no --offset
# is passed. Once this firmware is running the board no longer enumerates
# over USB (it has no USB stack), so re-flashing needs the bootloader:
# double-tap the reset button (the orange LED fades in and out) and re-run.
set -euo pipefail

ELF="${1:?usage: flash.sh <elf>}"
BIN="${ELF}.bin"

arm-none-eabi-objcopy -O binary "$ELF" "$BIN"
echo "== $(basename "$BIN"): $(wc -c < "$BIN" | tr -d ' ') bytes =="

# bossac: PATH first, then the Arduino IDE's bundled copy.
# An empty lookup is the expected "not on PATH" case and is handled by the
# Arduino15 glob and the explicit empty-BOSSAC check that follow.
BOSSAC="$(command -v bossac || true)"
if [ -z "$BOSSAC" ]; then
  for candidate in "$HOME"/Library/Arduino15/packages/arduino/tools/bossac/*/bossac; do
    if [ -x "$candidate" ]; then
      BOSSAC="$candidate"
      break
    fi
  done
fi
if [ -z "$BOSSAC" ]; then
  cat >&2 <<'EOF'
ERROR: bossac not found. Install one of:
  brew install bossa
  arduino-cli core install arduino:mbed_nano   (bundles bossac 1.9.1-arduino2)
EOF
  exit 1
fi

# First matching port, or empty. A bare unmatched glob stays literal, so test
# -e filters it; nothing here can fail under set -euo pipefail.
find_port() {
  local p
  for p in /dev/cu.usbmodem*; do
    if [ -e "$p" ]; then
      printf '%s\n' "$p"
      return 0
    fi
  done
  return 0
}

PORT="$(find_port)"
if [ -n "$PORT" ]; then
  # 1200-baud touch: asks a running Arduino sketch (or the bootloader's CDC)
  # to reboot into the bootloader. The port may keep the same name after the
  # touch, so only re-probe; do not require it to change. GNU coreutils stty
  # lacks -f, so call the BSD binary by absolute path on Darwin.
  if [ "$(uname -s)" = "Darwin" ]; then
    # A failed touch is genuinely expected (the port may already be the
    # bootloader, or may vanish mid-call as the board reboots); the empty
    # PORT after re-probe is handled explicitly by the check below.
    /bin/stty -f "$PORT" 1200 || true
  else
    # Same expected-failure rationale as the Darwin branch above; Linux stty
    # selects the device with -F rather than -f.
    stty -F "$PORT" 1200 || true
  fi
  sleep 3
  PORT="$(find_port)"
fi

if [ -z "$PORT" ]; then
  cat >&2 <<'EOF'
ERROR: no /dev/cu.usbmodem* port found.
Put the board in bootloader mode: double-tap the small reset button (the
orange LED will slowly fade in and out), then re-run:
  cargo run --release
EOF
  exit 1
fi

echo "== flashing via $PORT =="
"$BOSSAC" -d --port="$PORT" -U -i -e -w "$BIN" -R
echo "== flashed; the board reboots into the firmware (no USB enumeration is expected) =="

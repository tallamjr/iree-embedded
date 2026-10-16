#!/usr/bin/env bash
# Check a Nano timing log from timing_usb. Usage: validate-log.sh <log>
# Lines that start with "# " are the header and are skipped. The other lines
# must be exactly this, in this order, and nothing else:
#   header, then for cache off and for cache on: label, logits, cycles, microseconds.
# The label must be "yes", the logits must peak at the third value, and the
# cycle numbers must not decrease from min to max. Exit 0 when valid.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: validate-log.sh <log>" >&2
  exit 2
fi
if [ ! -f "$1" ]; then
  echo "ERROR: $1 is not a file" >&2
  exit 2
fi

awk '
function fail(message) {
  printf "ERROR: line %d: %s\n", NR, message > "/dev/stderr"
  failed = 1
  exit 1
}
function expect(kind) {
  if (sequence[step] != kind) {
    fail("expected " sequence[step] " line, found " kind " line: " $0)
  }
  step++
}
function check_stats(    fields, count) {
  count = split($0, fields, " ")
  min = fields[count - 6]; median = fields[count - 4]; p99 = fields[count - 2]; max = fields[count]
  if (min + 0 <= 0 || median + 0 < min + 0 || p99 + 0 < median + 0 || max + 0 < p99 + 0) {
    fail("numbers are not in order min <= median <= p99 <= max: " $0)
  }
}
BEGIN {
  split("header off_label off_logits off_cycles off_micros on_label on_logits on_cycles on_micros", sequence, " ")
  step = 1
}
/^# / { next }
/^$/ { next }
/ERROR|panick/ { fail("firmware reported an error: " $0) }
/^iree-embedded KWS timing: 1000 timed invokes$/ { expect("header"); next }
/^instruction cache (off|on): last invoke classified: / {
  if ($0 !~ /classified: yes \(expected .yes.\)$/) { fail("wrong label: " $0) }
  expect($3 == "off:" ? "off_label" : "on_label"); next
}
/^logits \[/ {
  if ($0 !~ /^logits \[-?[0-9]+\.[0-9]{4}, -?[0-9]+\.[0-9]{4}, -?[0-9]+\.[0-9]{4}, -?[0-9]+\.[0-9]{4}\]$/) {
    fail("bad logits line: " $0)
  }
  text = $0; gsub(/[^0-9.,-]/, "", text); split(text, value, ",")
  if (!(value[3] + 0 > value[1] + 0 && value[3] + 0 > value[2] + 0 && value[3] + 0 > value[4] + 0)) {
    fail("the third logit (yes) is not the largest: " $0)
  }
  expect(step == 3 ? "off_logits" : "on_logits"); next
}
/^cycles: min [0-9]+ median [0-9]+ p99 [0-9]+ max [0-9]+$/ {
  check_stats(); expect(step == 4 ? "off_cycles" : "on_cycles"); next
}
/^microseconds at 64000000 Hz: min [0-9]+ median [0-9]+ p99 [0-9]+ max [0-9]+$/ {
  check_stats(); expect(step == 5 ? "off_micros" : "on_micros"); next
}
{ fail("unexpected line: " $0) }
END {
  if (!failed && step != 10) {
    printf "ERROR: log is incomplete: %d of 9 expected lines found\n", step - 1 > "/dev/stderr"
    exit 1
  }
}
' "$1"

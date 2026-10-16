#!/usr/bin/env bash
# Test validate-log.sh. Usage: scripts/test-validate-log.sh
# The good log is built from the real micro:bit v2 log
# examples/microbit-v2-kws/results/timing_int8-run1.log: the same numbers,
# in the line format of timing_usb. The bad logs change one thing in the
# good log. Their names start with synthetic_.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real_log="$here/../../microbit-v2-kws/results/timing_int8-run1.log"
validator="$here/validate-log.sh"
work_dir="$(mktemp -d)"
passed=0
failed=0

# Turn the real RTT lines into timing_usb lines.
good="$work_dir/good.log"
{
  echo "# date_utc: 2026-01-01T00:00:00Z"
  grep -F "KWS int8 timing" "$real_log" | sed -E 's/^\[INFO \] iree-embedded KWS int8 timing:/iree-embedded KWS timing:/; s/ \(timing_int8.*$//'
  grep -E "instruction cache|^\[INFO \] cycles|microseconds|logits" "$real_log" |
    sed -E 's/^\[INFO \] //; s/ \(timing_int8 [^)]*\)$//' |
    sed -E 's/^logits \[([^]]*)\] \(reference.*$/logits [\1]/' |
    sed -E 's/^(logits \[)(.*)\]$/\1\2]/'
} > "$good"
# The real logits carry up to seven digits; timing_usb prints four.
perl -pi -e 's/(-?\d+\.\d{4})\d+/$1/g if /^logits/' "$good"

expect_result() {
  local name="$1" expected="$2" file="$3" status=0
  "$validator" "$file" > /dev/null || status=$?
  if { [ "$expected" = "pass" ] && [ "$status" -eq 0 ]; } || { [ "$expected" = "fail" ] && [ "$status" -eq 1 ]; }; then
    passed=$((passed + 1))
    echo "ok   $name"
  else
    failed=$((failed + 1))
    echo "FAIL $name (exit status $status)"
  fi
}

mutate() {
  local name="$1" expression="$2"
  perl -pe "$expression" "$good" > "$work_dir/$name.log"
}

expect_result real_numbers_in_usb_format pass "$good"

mutate synthetic_wrong_label 's/classified: yes/classified: no/ if $. == 3'
expect_result synthetic_wrong_label fail "$work_dir/synthetic_wrong_label.log"

mutate synthetic_yes_logit_not_largest 's/^logits \[([^,]+), ([^,]+), [^,]+,/logits [$1, $2, -9.0000,/'
expect_result synthetic_yes_logit_not_largest fail "$work_dir/synthetic_yes_logit_not_largest.log"

mutate synthetic_missing_first_cycles_line '$_ = "" if /^cycles:/ && !$seen++'
expect_result synthetic_missing_first_cycles_line fail "$work_dir/synthetic_missing_first_cycles_line.log"

mutate synthetic_missing_microseconds_line '$_ = "" if /^microseconds/ && ++$count == 2'
expect_result synthetic_missing_microseconds_line fail "$work_dir/synthetic_missing_microseconds_line.log"

mutate synthetic_median_below_min 's/median \d+/median 1/ if /^cycles:/'
expect_result synthetic_median_below_min fail "$work_dir/synthetic_median_below_min.log"

mutate synthetic_firmware_error '$_ .= "ERROR: failed: Allocate\n" if eof'
expect_result synthetic_firmware_error fail "$work_dir/synthetic_firmware_error.log"

mutate synthetic_unknown_line '$_ .= "noise\n" if eof'
expect_result synthetic_unknown_line fail "$work_dir/synthetic_unknown_line.log"

head -n 5 "$good" > "$work_dir/synthetic_cut_short.log"
expect_result synthetic_cut_short fail "$work_dir/synthetic_cut_short.log"

: > "$work_dir/synthetic_empty.log"
expect_result synthetic_empty fail "$work_dir/synthetic_empty.log"

echo "$passed passed, $failed failed"
[ "$failed" -eq 0 ]

#!/usr/bin/env bash
# Test validate-log.sh. Usage: scripts/test-validate-log.sh
# The good log is built from the real micro:bit v2 log
# examples/microbit-v2-kws/results/timing_int8-run1.log: the same numbers,
# in the line format of timing_usb (logits rounded to four decimals, as the
# firmware formatter does). The bad logs change one thing in the good log.
# Their names start with synthetic_. Each must fail with its own message.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real_log="$here/../../microbit-v2-kws/results/timing_int8-run1.log"
validator="$here/validate-log.sh"
# A fixed folder: the safety hook blocks deleting a variable path, so the
# script reuses this folder and overwrites its files instead of cleaning up.
work_dir="${TMPDIR:-/tmp}/nano-validate-log-test"
mkdir -p "$work_dir"
passed=0
failed=0

good="$work_dir/good.log"
{
  echo "# date_utc: 2026-01-01T00:00:00Z"
  grep -F "KWS int8 timing" "$real_log" | sed -E 's/^\[INFO \] iree-embedded KWS int8 timing:/iree-embedded KWS timing:/; s/ \(timing_int8.*$//'
  grep -E "instruction cache|^\[INFO \] cycles|microseconds|logits" "$real_log" |
    sed -E 's/^\[INFO \] //; s/ \(timing_int8 [^)]*\)$//' |
    sed -E 's/^logits \[([^]]*)\] \(reference.*$/logits [\1]/'
} > "$good"
perl -pi -e 'if (/^logits \[(.*)\]$/) { $_ = "logits [" . join(", ", map { sprintf("%.4f", $_) } split(/, /, $1)) . "]\n" }' "$good"

# Usage: expect_result <name> <pass|fail> <file> [message-pattern]
expect_result() {
  local name="$1" expected="$2" file="$3" pattern="${4:-}" status=0 message
  message="$("$validator" "$file" 2>&1)" || status=$?
  if [ "$expected" = "pass" ] && [ "$status" -eq 0 ]; then
    passed=$((passed + 1))
    echo "ok   $name"
  elif [ "$expected" = "fail" ] && [ "$status" -eq 1 ] && grep -q -E -- "$pattern" <<< "$message"; then
    passed=$((passed + 1))
    echo "ok   $name"
  else
    failed=$((failed + 1))
    echo "FAIL $name (exit status $status): $message"
  fi
}

# Usage: expect_mutation <name> <perl-expression> <message-pattern>
expect_mutation() {
  local name="$1" expression="$2" pattern="$3"
  perl -pe "$expression" "$good" > "$work_dir/$name.log"
  expect_result "$name" fail "$work_dir/$name.log" "$pattern"
}

expect_result real_numbers_in_usb_format pass "$good"

expect_mutation synthetic_wrong_label 's/classified: yes/classified: no/ if $. == 3' 'wrong label'
expect_mutation synthetic_yes_logit_not_largest 's/^logits \[([^,]+), ([^,]+), [^,]+,/logits [$1, $2, -9.0000,/' 'third logit'
expect_mutation synthetic_logits_with_seven_decimals 's/^(logits \[-?\d+\.\d{4})/${1}123/ if /^logits/ && !$seen++' 'bad logits line'
expect_mutation synthetic_missing_first_cycles_line '$_ = "" if /^cycles:/ && !$seen++' 'expected off_cycles'
expect_mutation synthetic_missing_microseconds_line '$_ = "" if /^microseconds/ && ++$count == 2' 'incomplete'
expect_mutation synthetic_median_below_min 's/median \d+/median 1/ if /^cycles:/' 'not in order'
expect_mutation synthetic_firmware_error '$_ .= "ERROR: failed: Allocate\n" if eof' 'firmware reported an error'
expect_mutation synthetic_unknown_line '$_ .= "noise\n" if eof' 'unexpected line'

head -n 5 "$good" > "$work_dir/synthetic_cut_short.log"
expect_result synthetic_cut_short fail "$work_dir/synthetic_cut_short.log" 'incomplete'

: > "$work_dir/synthetic_empty.log"
expect_result synthetic_empty fail "$work_dir/synthetic_empty.log" 'incomplete'

echo "$passed passed, $failed failed"
[ "$failed" -eq 0 ]

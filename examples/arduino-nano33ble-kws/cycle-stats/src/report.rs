//! Text lines of the timing result. The USB serial firmware prints them and
//! `scripts/validate-log.sh` reads them.
use core::fmt::{self, Write};

use crate::{CycleStats, cycles_to_micros};

/// Label the model must give for the embedded clip.
pub const EXPECTED_LABEL: &str = "yes";

/// Result of one timed run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CacheRun<'a> {
    pub cache_on: bool,
    pub label: &'a str,
    pub logits: [f32; 4],
    pub stats: CycleStats,
}

/// First line of the result text.
pub fn write_header<W: Write>(out: &mut W, timed_invokes: usize) -> fmt::Result {
    writeln!(
        out,
        "iree-embedded KWS timing: {timed_invokes} timed invokes"
    )
}

/// The four lines of one run: label, logits, cycles and microseconds.
pub fn write_cache_run<W: Write>(out: &mut W, run: &CacheRun, clock_hz: u32) -> fmt::Result {
    let cache = if run.cache_on { "on" } else { "off" };
    let [first, second, third, fourth] = run.logits;
    let stats = run.stats;
    writeln!(
        out,
        "instruction cache {cache}: last invoke classified: {} (expected '{EXPECTED_LABEL}')",
        run.label
    )?;
    writeln!(
        out,
        "logits [{first:.4}, {second:.4}, {third:.4}, {fourth:.4}]"
    )?;
    writeln!(
        out,
        "cycles: min {} median {} p99 {} max {}",
        stats.min, stats.median, stats.p99, stats.max
    )?;
    writeln!(
        out,
        "microseconds at {clock_hz} Hz: min {} median {} p99 {} max {}",
        cycles_to_micros(stats.min, clock_hz),
        cycles_to_micros(stats.median, clock_hz),
        cycles_to_micros(stats.p99, clock_hz),
        cycles_to_micros(stats.max, clock_hz)
    )
}

/// Fixed-size text buffer. A write that does not fit fails with `fmt::Error`.
pub struct TextBuffer<const CAPACITY: usize> {
    bytes: [u8; CAPACITY],
    length: usize,
}

impl<const CAPACITY: usize> TextBuffer<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; CAPACITY],
            length: 0,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}

impl<const CAPACITY: usize> Default for TextBuffer<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> Write for TextBuffer<CAPACITY> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.length + text.len();
        if end > CAPACITY {
            return Err(fmt::Error);
        }
        self.bytes[self.length..end].copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Numbers from examples/microbit-v2-kws/results/timing_int8-run1.log.
    const CACHE_OFF: CacheRun = CacheRun {
        cache_on: false,
        label: "yes",
        logits: [-5.870843, -1.6511745, 9.815315, -1.6511745],
        stats: CycleStats {
            min: 7279930,
            median: 7279930,
            p99: 7279930,
            max: 7279930,
        },
    };
    const CACHE_ON: CacheRun = CacheRun {
        cache_on: true,
        label: "yes",
        logits: [-5.870843, -1.6511745, 9.815315, -1.6511745],
        stats: CycleStats {
            min: 6233349,
            median: 6233349,
            p99: 6233349,
            max: 6233355,
        },
    };

    fn text_of<const N: usize>(buffer: &TextBuffer<N>) -> &str {
        core::str::from_utf8(buffer.as_bytes()).unwrap()
    }

    #[test]
    fn header_line() {
        let mut buffer = TextBuffer::<128>::new();
        write_header(&mut buffer, 1000).unwrap();
        assert_eq!(
            text_of(&buffer),
            "iree-embedded KWS timing: 1000 timed invokes\n"
        );
    }

    #[test]
    fn cache_off_run_lines() {
        let mut buffer = TextBuffer::<512>::new();
        write_cache_run(&mut buffer, &CACHE_OFF, 64_000_000).unwrap();
        assert_eq!(
            text_of(&buffer),
            "instruction cache off: last invoke classified: yes (expected 'yes')\n\
             logits [-5.8708, -1.6512, 9.8153, -1.6512]\n\
             cycles: min 7279930 median 7279930 p99 7279930 max 7279930\n\
             microseconds at 64000000 Hz: min 113748 median 113748 p99 113748 max 113748\n"
        );
    }

    #[test]
    fn cache_on_run_lines() {
        let mut buffer = TextBuffer::<512>::new();
        write_cache_run(&mut buffer, &CACHE_ON, 64_000_000).unwrap();
        assert_eq!(
            text_of(&buffer),
            "instruction cache on: last invoke classified: yes (expected 'yes')\n\
             logits [-5.8708, -1.6512, 9.8153, -1.6512]\n\
             cycles: min 6233349 median 6233349 p99 6233349 max 6233355\n\
             microseconds at 64000000 Hz: min 97396 median 97396 p99 97396 max 97396\n"
        );
    }

    #[test]
    fn wrong_label_is_printed_as_given() {
        let wrong = CacheRun {
            label: "no",
            ..CACHE_ON
        };
        let mut buffer = TextBuffer::<512>::new();
        write_cache_run(&mut buffer, &wrong, 64_000_000).unwrap();
        assert!(
            text_of(&buffer)
                .starts_with("instruction cache on: last invoke classified: no (expected 'yes')\n")
        );
    }

    #[test]
    fn buffer_rejects_text_that_does_not_fit() {
        let mut buffer = TextBuffer::<8>::new();
        assert!(buffer.write_str("12345678").is_ok());
        assert!(buffer.write_str("9").is_err());
        assert_eq!(buffer.as_bytes(), b"12345678");
    }

    #[test]
    fn full_result_fits_the_firmware_buffer() {
        let mut buffer = TextBuffer::<1024>::new();
        write_header(&mut buffer, 1000).unwrap();
        write_cache_run(&mut buffer, &CACHE_OFF, 64_000_000).unwrap();
        write_cache_run(&mut buffer, &CACHE_ON, 64_000_000).unwrap();
        assert!(buffer.as_bytes().len() < 1024);
    }
}

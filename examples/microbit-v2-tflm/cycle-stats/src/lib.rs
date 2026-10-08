//! Min, median, p99 and max of a set of cycle counts.
//!
//! Run the tests on the host: `cargo test -p cycle-stats --target aarch64-apple-darwin`.
#![no_std]
#![forbid(unsafe_code)]

/// Summary of a set of cycle counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleStats {
    pub min: u32,
    pub median: u32,
    pub p99: u32,
    pub max: u32,
}

/// Sorts `cycles` in place and summarises it. Returns `None` for an empty slice.
///
/// The median of an even count is the mean of the two middle values, rounded down.
/// The p99 is the nearest-rank value: the sorted element at index `ceil(0.99 * n) - 1`.
pub fn summarise(cycles: &mut [u32]) -> Option<CycleStats> {
    let count = cycles.len();
    if count == 0 {
        return None;
    }
    cycles.sort_unstable();
    let median = if count % 2 == 1 {
        cycles[count / 2]
    } else {
        let sum = u64::from(cycles[count / 2 - 1]) + u64::from(cycles[count / 2]);
        (sum / 2) as u32
    };
    let p99_index = (count * 99).div_ceil(100) - 1;
    Some(CycleStats {
        min: cycles[0],
        median,
        p99: cycles[p99_index],
        max: cycles[count - 1],
    })
}

/// Converts cycles to microseconds, rounded down. `clock_hz` must be above zero.
pub fn cycles_to_micros(cycles: u32, clock_hz: u32) -> u64 {
    u64::from(cycles) * 1_000_000 / u64::from(clock_hz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_slice_has_no_summary() {
        assert_eq!(summarise(&mut []), None);
    }

    #[test]
    fn single_element_fills_every_field() {
        let stats = summarise(&mut [42]).unwrap();
        assert_eq!(
            stats,
            CycleStats {
                min: 42,
                median: 42,
                p99: 42,
                max: 42
            }
        );
    }

    #[test]
    fn odd_length_median_is_middle_value() {
        let stats = summarise(&mut [5, 1, 3]).unwrap();
        assert_eq!(stats.median, 3);
    }

    #[test]
    fn even_length_median_is_mean_of_middle_pair() {
        let stats = summarise(&mut [4, 1, 3, 2]).unwrap();
        assert_eq!(stats.median, 2);
        let stats = summarise(&mut [1, 2, 3, 6]).unwrap();
        assert_eq!(stats.median, 2);
    }

    #[test]
    fn even_median_does_not_overflow_u32() {
        let stats = summarise(&mut [u32::MAX, u32::MAX]).unwrap();
        assert_eq!(stats.median, u32::MAX);
    }

    #[test]
    fn unsorted_input_gives_min_and_max() {
        let stats = summarise(&mut [9, 2, 7, 100, 1, 50]).unwrap();
        assert_eq!(stats.min, 1);
        assert_eq!(stats.max, 100);
    }

    #[test]
    fn input_slice_is_left_sorted() {
        let mut cycles = [3, 1, 2];
        summarise(&mut cycles).unwrap();
        assert_eq!(cycles, [1, 2, 3]);
    }

    #[test]
    fn p99_of_1000_values_is_index_989() {
        let mut cycles: [u32; 1000] = core::array::from_fn(|i| 1000 - i as u32);
        let stats = summarise(&mut cycles).unwrap();
        assert_eq!(stats.p99, 990);
        assert_eq!(stats.max, 1000);
    }

    #[test]
    fn p99_of_100_values_is_the_99th_value() {
        let mut cycles: [u32; 100] = core::array::from_fn(|i| i as u32 + 1);
        assert_eq!(summarise(&mut cycles).unwrap().p99, 99);
    }

    #[test]
    fn p99_of_small_sets_is_the_max() {
        assert_eq!(summarise(&mut [1, 2, 3]).unwrap().p99, 3);
        assert_eq!(summarise(&mut [7, 9]).unwrap().p99, 9);
    }

    #[test]
    fn conversion_at_64_mhz() {
        assert_eq!(cycles_to_micros(64, 64_000_000), 1);
        assert_eq!(cycles_to_micros(64_000_000, 64_000_000), 1_000_000);
        assert_eq!(cycles_to_micros(0, 64_000_000), 0);
    }

    #[test]
    fn conversion_rounds_down() {
        assert_eq!(cycles_to_micros(95, 64_000_000), 1);
    }

    #[test]
    fn conversion_handles_large_counts_without_overflow() {
        assert_eq!(cycles_to_micros(u32::MAX, 64_000_000), 67_108_863);
    }
}

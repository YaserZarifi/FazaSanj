//! FILETIME conversion.

/// 100 ns ticks between 1601-01-01 and 1970-01-01.
const EPOCH_DIFF: i64 = 116_444_736_000_000_000;

/// FILETIME (100 ns ticks since 1601) to unix milliseconds. Zero means "not set".
pub fn filetime_to_unix_ms(ft: i64) -> Option<i64> {
    if ft <= 0 {
        return None;
    }
    Some((ft - EPOCH_DIFF).div_euclid(10_000))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        assert_eq!(filetime_to_unix_ms(EPOCH_DIFF), Some(0));
        assert_eq!(filetime_to_unix_ms(EPOCH_DIFF + 10_000), Some(1));
        assert_eq!(filetime_to_unix_ms(0), None);
    }
}

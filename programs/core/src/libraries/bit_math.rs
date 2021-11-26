
/// Returns index of the most significant non-zero bit of the number
///
/// The function satisfies the property:
///     x >= 2**most_significant_bit(x) and x < 2**(most_significant_bit(x)+1)
///
///
/// # Arguments
///
/// * `x` - the value for which to compute the most significant bit, must be greater than 0
///
pub fn most_significant_bit(mut x: u64) -> u8 {
    assert!(x > 0);

    let mut msb = 0; // in [1, 64)

    let mut f: u8 = ((x >= 0x100000000) as u8) << 5; // If r >= 2^32, f = 32 else 0
    msb |= f; // Add f to MSB
    x >>= f; // Right shift by f

    f = ((x >= 0x10000) as u8) << 4; // 2^16
    msb |= f;
    x >>= f;

    f = ((x >= 0x100) as u8) << 3; // 2^8
    msb |= f;
    x >>= f;

    f = ((x >= 0x10) as u8) << 2; // 2^4
    msb |= f;
    x >>= f;

    f = ((x >= 0x4) as u8) << 1; // 2^2
    msb |= f;
    x >>= f;

    f = ((x >= 0x2) as u8) << 0; // 2^0
    msb |= f;

    msb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msb_at_powers_of_two() {
        for i in 0..63 {
            assert_eq!(most_significant_bit(u64::pow(2, i)), i as u8);
            // entire 0..u64::MAX range takes too much time to test
        }
    }
}

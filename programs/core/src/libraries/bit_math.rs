/// Helper functions to get most and least significant non-zero bits

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

/// Returns index of the least significant non-zero bit of the number
///
/// The function satisfies the property:
///     (x & 2**leastSignificantBit(x)) != 0 and (x & (2**(leastSignificantBit(x)) - 1)) == 0)
///
///
/// # Arguments
///
/// * `x` - the value for which to compute the least significant bit, must be greater than 0
///
pub fn least_significant_bit(mut x: u64) -> u8 {
    assert!(x > 0);

    let mut lsb: u8 = 63;
    if x & 0xffffffff > 0 {
        // u32::MAX
        lsb -= 32;
    } else {
        x >>= 32;
    }
    if x & 0xffff > 0 {
        // u16::MAX
        lsb -= 16;
    } else {
        x >>= 16;
    }
    if x & 0xff > 0 {
        // u8::MAX
        lsb -= 8;
    } else {
        x >>= 8;
    }
    if x & 0xf > 0 {
        // u4::MAX
        lsb -= 4;
    } else {
        x >>= 4;
    }
    if x & 0x3 > 0 {
        // u2::MAX
        lsb -= 2;
    } else {
        x >>= 2;
    }
    if x & 0x1 > 0 {
        // u1::MAX
        lsb -= 1;
    }

    lsb
}

#[cfg(test)]
mod significant_bits {
    use super::*;

    // mostSignificantBit
    mod most_significant_bit {
        use super::*;

        #[test]
        fn test_msb_at_powers_of_two() {
            for i in 0..63 {
                assert_eq!(most_significant_bit(u64::pow(2, i)), i as u8);
                // entire 0..u64::MAX range takes too much time to test
            }
        }

        #[test]
        #[should_panic]
        fn test_msb_for_0() {
            most_significant_bit(0);
        }

        #[test]
        fn test_msb_for_1() {
            assert_eq!(most_significant_bit(1), 0);
        }

        #[test]
        fn test_msb_for_2() {
            assert_eq!(most_significant_bit(2), 1);
        }

        #[test]
        fn test_msb_for_max() {
            assert_eq!(
                most_significant_bit(u64::MAX.checked_sub(1u64).unwrap()),
                63
            );
        }

        #[test]
        fn within_limits() {
            for i in 0..63 {
                let input = u64::pow(2, i);
                let msb = most_significant_bit(input);
                assert!(input >= u64::pow(2, msb.into()));
                // entire 0..u64::MAX range takes too much time to test
            }
        }
    }

    // leastSignificantBit
    mod least_significant_bit {
        use super::*;

        #[test]
        fn test_at_powers_of_two() {
            for i in 0..63 {
                assert_eq!(least_significant_bit(u64::pow(2, i)), i as u8);
                // entire 0..u64::MAX range takes too much time to test
            }
        }

        #[test]
        #[should_panic]
        fn test_for_0() {
            least_significant_bit(0);
        }

        #[test]
        fn test_for_1() {
            assert_eq!(least_significant_bit(1), 0);
        }

        #[test]
        fn test_for_2() {
            assert_eq!(least_significant_bit(2), 1);
        }

        #[test]
        fn test_for_max() {
            assert_eq!(
                least_significant_bit(u64::MAX.checked_sub(1u64).unwrap()),
                1
            );
        }

        #[test]
        fn within_limits() {
            for i in 0..63 {
                let input = u64::pow(2, i);
                let lsb = least_significant_bit(input);
                assert!(input & u64::pow(2, lsb.into()) != 0);
                // entire 0..u64::MAX range takes too much time to test
            }
        }
    }
}

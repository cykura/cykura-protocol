///! Helper functions to calculate tick from √P and vice versa
///! Performs power and log calculations in a gas efficient manner
///!
///! # Resources
///!
///! * https://medium.com/coinmonks/math-in-solidity-part-5-exponent-and-logarithm-9aef8515136e
///! * https://liaoph.com/logarithm-in-solidity/
///!
use anchor_lang::require;
use crate::error::ErrorCode;

pub const MIN_TICK: i32 = -221818;
pub const MAX_TICK: i32 = -MIN_TICK;

pub const MIN_SQRT_RATIO: u64 = 65536; // 2^32
pub const MAX_SQRT_RATIO: u64 = 281474976710656; // 2^48

/// Calculates 1.0001^(tick/2) as a U32.32 number representing
/// the square root of the ratio of the two assets (token_1/token_0)
///
/// Calculates result as a U64.64, then rounds down to U32.32.
/// Each magic factor is `2^64 / (1.0001^(2^(i - 1)))` for i in `[0, 18)`.
///
/// Uniswap follows `2^128 / (1.0001^(2^(i - 1)))` for i in [0, 20), for U128.128
///
/// Throws if |tick| > MAX_TICK
///
/// # Arguments
/// * `tick` - Price tick
///
pub fn get_sqrt_ratio_at_tick(tick: i32) -> Result<u64, ErrorCode> {
    let abs_tick = tick.abs() as u128;
    require!(abs_tick <= MAX_TICK as u128, ErrorCode::T);

    // i = 0
    let mut ratio: u128 = if abs_tick & 0x1 != 0 {
        0xfffcb933bd6fb800
    } else {
        // 2^64
        0x10000000000000000
    };
    // i = 1
    if abs_tick & 0x2 != 0 { ratio = (ratio * 0xfff97272373d4000) >> 64 };
    // i = 2
    if abs_tick & 0x4 != 0 { ratio = (ratio * 0xfff2e50f5f657000) >> 64 };
    // i = 3
    if abs_tick & 0x8 != 0 { ratio = (ratio * 0xffe5caca7e10f000) >> 64 };
    // i = 4
    if abs_tick & 0x10 != 0 { ratio = (ratio * 0xffcb9843d60f7000) >> 64 };
    // i = 5
    if abs_tick & 0x20 != 0 { ratio = (ratio * 0xff973b41fa98e800) >> 64 };
    // i = 6
    if abs_tick & 0x40 != 0 { ratio = (ratio * 0xff2ea16466c9b000) >> 64 };
    // i = 7
    if abs_tick & 0x80 != 0 { ratio = (ratio * 0xfe5dee046a9a3800) >> 64 };
    // i = 8
    if abs_tick & 0x100 != 0 { ratio = (ratio * 0xfcbe86c7900bb000) >> 64 };
    // i = 9
    if abs_tick & 0x200 != 0 { ratio = (ratio * 0xf987a7253ac65800) >> 64 };
    // i = 10
    if abs_tick & 0x400 != 0 { ratio = (ratio * 0xf3392b0822bb6000) >> 64 };
    // i = 11
    if abs_tick & 0x800 != 0 { ratio = (ratio * 0xe7159475a2caf000) >> 64 };
    // i = 12
    if abs_tick & 0x1000 != 0 { ratio = (ratio * 0xd097f3bdfd2f2000) >> 64 };
    // i = 13
    if abs_tick & 0x2000 != 0 { ratio = (ratio * 0xa9f746462d9f8000) >> 64 };
    // i = 14
    if abs_tick & 0x4000 != 0 { ratio = (ratio * 0x70d869a156f31c00) >> 64 };
    // i = 15
    if abs_tick & 0x8000 != 0 { ratio = (ratio * 0x31be135f97ed3200) >> 64 };
    // i = 16
    if abs_tick & 0x10000 != 0 { ratio = (ratio * 0x9aa508b5b85a500) >> 64 };
    // i = 17
    if abs_tick & 0x20000 != 0 { ratio = (ratio * 0x5d6af8dedc582c) >> 64 };

    // Divide to obtain 1.0001^(2^(i - 1)) * 2^32 in numerator
    if tick > 0 {
        ratio = u128::MAX / ratio;
    }

    // Rounding up and convert to U32.32
    let sqrt_price_x32 = ((ratio >> 32) as u64) + (((ratio % (1 << 32) != 0) as u64));

    Ok(sqrt_price_x32)
}

/// Calculates the greatest tick value such that get_sqrt_ratio_at_tick(tick) <= ratio
/// Throws if sqrt_price_x32 < MIN_SQRT_RATIO or sqrt_price_x32 > MAX_SQRT_RATIO
///
/// Formula: `i = log base(√1.0001) (√P)`
///
/// # Arguments
///
/// * `sqrt_price_x32`- The sqrt ratio for which to compute the tick as a U32.32
///
pub fn get_tick_at_sqrt_ratio(sqrt_price_x32: u64) -> Result<i32, ErrorCode> {
    // second inequality must be < because the price can never reach the price at the max tick
    require!(sqrt_price_x32 >= MIN_SQRT_RATIO && sqrt_price_x32 < MAX_SQRT_RATIO, ErrorCode::R);

    let mut r = sqrt_price_x32;
    let mut msb = 0; // in [1, 64)

    // ------------------------------------------------------
    // Decimal part of logarithm = MSB
    // Binary search method: 2^32, 2^16, 2^8, 2^4, 2^2 and 2^1 for U32.32

    let mut f: u8 = ((r >= 0x100000000) as u8) << 5; // If r >= 2^32, f = 32 else 0
    msb |= f; // Add f to MSB
    r >>= f; // Right shift by f

    f = ((r >= 0x10000) as u8) << 4; // 2^16
    msb |= f;
    r >>= f;

    f = ((r >= 0x100) as u8) << 3; // 2^8
    msb |= f;
    r >>= f;

    f = ((r >= 0x10) as u8) << 2; // 2^4
    msb |= f;
    r >>= f;

    f = ((r >= 0x4) as u8) << 1; // 2^2
    msb |= f;
    r >>= f;

    f = ((r >= 0x2) as u8) << 0; // 2^0
    msb |= f;

    // log2 (m x 2^e) = log2 (m) + e
    // For U32.32, e = -32. Subtract by 32 to remove x32 notation.
    // Then left shift by 16 bits to convert into U48.16 form
    let mut log_2_x16 = (msb as i64 - 32) << 16;

    // ------------------------------------------------------
    // Fractional part of logarithm

    // Set r = r / 2^n as a Q33.31 number, where n stands for msb
    r = if msb >= 32 {
        sqrt_price_x32 >> (msb - 31)
    } else {
        sqrt_price_x32 << (31 - msb)
    };

    r = (r*r) >> 31; // r^2 as U33.31
    f = (r >> 32) as u8; // MSB of r^2 (0 or 1)
    log_2_x16 |= (f as i64) << 15; // Add f at 1st fractional place
    r >>= f; // Divide r by 2 if MSB of f is non-zero

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 14;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 13;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 12;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 11;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 10;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 9;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 8;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 7;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 6;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 5;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 4;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 3;
    r >>= f;

    r = (r*r) >> 31;
    f = (r >> 32) as u8;
    log_2_x16 |= (f as i64) << 2;

    // 14 bit refinement gives an error margin of 2^-14 / log2 (√1.0001) = 0.8461 < 1
    // Since tick is a decimal, an error under 1 is acceptable

    // Change of base rule: multiply with 2^16 / log2 (√1.0001)
    let log_sqrt_10001_x32 = log_2_x16 * 908567298;

    // tick - 0.01
    let tick_low = ((log_sqrt_10001_x32 - 42949672) >> 32) as i32;

    // tick + (2^-14 / log2(√1.001)) + 0.01
    let tick_high = ((log_sqrt_10001_x32 + 3677218864) >> 32) as i32;

    Ok(if tick_low == tick_high {
        tick_low
    } else if get_sqrt_ratio_at_tick(tick_high).unwrap() <= sqrt_price_x32 {
        tick_high
    } else {
        tick_low
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_price_error_under_1_bps() {
        for tick in MIN_TICK..=MAX_TICK {
            let sqrt_price_x32 = get_sqrt_ratio_at_tick(tick).unwrap();
            let sqrt_price = (sqrt_price_x32 as f64) / (4294967296.0);
            let float_price = f64::powf(1.0001, (tick as f64) / 2.0);

            // Error should be under 0.01%
            let deviation = (sqrt_price - float_price) / float_price;
            assert!(deviation.abs() < 0.0001);
        }
    }

    #[test]
    fn sqrt_price_increases_with_tick() {
        let mut prev_price_x32: u64 = 0;
        for tick in MIN_TICK..=MAX_TICK {
            let sqrt_price_x32 = get_sqrt_ratio_at_tick(tick).unwrap();
            // P should increase with tick
            if prev_price_x32 != 0 {
                assert!(sqrt_price_x32 > prev_price_x32);
            }
            prev_price_x32 = sqrt_price_x32;
        }
    }

    #[test]
    fn retrieve_original_tick() {
        for tick in MIN_TICK..=MAX_TICK {
            let sqrt_price_x32 = get_sqrt_ratio_at_tick(tick).unwrap();

            // Original tick should be obtained by operating on √P
            let obtained_tick = get_tick_at_sqrt_ratio(sqrt_price_x32).unwrap();
            assert_eq!(tick, obtained_tick, "Tick {}, obtained tick {}", tick, obtained_tick);
        }
    }

    #[test]
    #[should_panic]
    fn less_than_min_tick() {
        get_sqrt_ratio_at_tick(MIN_TICK - 1).unwrap();
    }

    #[test]
    #[should_panic]
    fn greater_than_min_tick() {
        get_sqrt_ratio_at_tick(MAX_TICK + 1).unwrap();
    }

    #[test]
    #[should_panic]
    fn less_than_min_sqrt_ratio() {
        get_tick_at_sqrt_ratio(MIN_SQRT_RATIO - 1).unwrap();
    }

    #[test]
    #[should_panic]
    fn greater_than_or_equal_to_max_sqrt_ratio() {
        get_tick_at_sqrt_ratio(MAX_SQRT_RATIO).unwrap();
    }

    #[test]
    fn ratio_of_min_tick() {
        let tick = MIN_TICK - 1;
        let obtained_ratio = get_tick_at_sqrt_ratio(MIN_SQRT_RATIO).unwrap();
        assert_eq!(tick, obtained_ratio, "Tick {}, obtained ratio {}", tick, obtained_ratio);
    }

    #[test]
    fn ratio_closest_to_max_tick() {
        let obtained_ratio = get_tick_at_sqrt_ratio(MAX_SQRT_RATIO - 1).unwrap();
        assert_eq!(MAX_TICK, obtained_ratio, "Tick {}, obtained ratio {}", MAX_TICK, obtained_ratio);
    }
}

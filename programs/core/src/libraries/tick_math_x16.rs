pub const MIN_TICK: i32 = -221_818;
pub const MAX_TICK: i32 = -MIN_TICK;

pub const MIN_SQRT_PRICE: u64 = 65536;
pub const MAX_SQRT_PRICE: u64 = 281474976710655;

/// Calculates 1.0001^(tick/2) as a U32.32 number representing
/// the square root of the ratio of the two assets (token_1/token_0)
///
/// Throws if |tick| > MAX_TICK
///
/// # Math
///
/// * A gas efficient implementation to find powers of sqrt(1.0001).
///  It find bitwise exponents with pre-computed magic factors.
///
/// * Each magic factor is `2^32 / (1.0001^(2^(i - 1)))` for i in `[0, 18)`
///
/// * Uniswap follows `2^128 / (1.0001^(2^(i - 1)))` for i in [0, 20), for U128.128
///
/// # Arguments
/// * `tick` - Price tick
///
pub fn sqrt_price_x32(tick: i32) -> u64 {
    let abs_tick = tick.abs() as u64;
    assert!(abs_tick <= MAX_TICK as u64);

    // i = 0
    let mut ratio: u64 = if abs_tick & 0x1 != 0 {
        0xfffcb934
    } else {
        // 2^48
        0x100000000
    };
    // i = 1, p =
    if abs_tick & 0x2 != 0 { ratio = (ratio * 0xfff97272) >> 32 };
    // i = 2
    if abs_tick & 0x4 != 0 { ratio = (ratio * 0xfff2e50f) >> 32 };
    // i = 3
    if abs_tick & 0x8 != 0 { ratio = (ratio * 0xffe5caca) >> 32 };
    // i = 4
    if abs_tick & 0x10 != 0 { ratio = (ratio * 0xffcb9844) >> 32 };
    // i = 5
    if abs_tick & 0x20 != 0 { ratio = (ratio * 0xff973b42) >> 32 };
    // i = 6
    if abs_tick & 0x40 != 0 { ratio = (ratio * 0xff2ea164) >> 32 };
    // i = 7
    if abs_tick & 0x80 != 0 { ratio = (ratio * 0xfe5dee04) >> 32 };
    // i = 8
    if abs_tick & 0x100 != 0 { ratio = (ratio * 0xfcbe86c8) >> 32 };
    // i = 9
    if abs_tick & 0x200 != 0 { ratio = (ratio * 0xf987a725) >> 32 };
    // i = 10
    if abs_tick & 0x400 != 0 { ratio = (ratio * 0xf3392b08) >> 32 };
    // i = 11
    if abs_tick & 0x800 != 0 { ratio = (ratio * 0xe7159476) >> 32 };
    // i = 12
    if abs_tick & 0x1000 != 0 { ratio = (ratio * 0xd097f3be) >> 32 };
    // i = 13
    if abs_tick & 0x2000 != 0 { ratio = (ratio * 0xa9f74646) >> 32 };
    // i = 14
    if abs_tick & 0x4000 != 0 { ratio = (ratio * 0x70d869a1) >> 32 };
    // i = 15
    if abs_tick & 0x8000 != 0 { ratio = (ratio * 0x31be1360) >> 32 };
    // i = 16
    if abs_tick & 0x10000 != 0 { ratio = (ratio * 0x9aa508b) >> 32 };
    // i = 17
    if abs_tick & 0x20000 != 0 { ratio = (ratio * 0x5d6af9) >> 32 };

    // Divide to obtain 1.0001^(2^(i - 1)) * 2^32 in numerator
    if tick > 0 {
        ratio = u64::MAX / ratio;
    }

    ratio
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tick() {
        let mut prev_price_x32: u64 = 0;
        for tick in MIN_TICK..=MAX_TICK {
            let sqrt_price_x32 = sqrt_price_x32(tick);
            let sqrt_price = (sqrt_price_x32 as f64) / (4294967296.0);
            let float_price = f64::powf(1.0001, (tick as f64) / 2.0);
            let deviation = (sqrt_price - float_price) / float_price;

            // println!("i {}, Approx x32 {}, approx {}, actual {}, deviation {}", tick, sqrt_price_x32, sqrt_price, float_price, deviation);

            assert!(deviation.abs() < 0.0001); // 0.01% error margin
            if prev_price_x32 != 0 {
                assert!(sqrt_price_x32 > prev_price_x32);
            }
            prev_price_x32 = sqrt_price_x32;
        }
    }

    #[test]
    #[should_panic]
    fn less_than_min_tick() {
        sqrt_price_x32(MIN_TICK - 1);
    }

    #[test]
    #[should_panic]
    fn greater_than_min_tick() {
        sqrt_price_x32(MAX_TICK + 1);
    }
}


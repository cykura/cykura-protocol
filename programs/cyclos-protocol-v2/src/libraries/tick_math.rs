/// Convert tick i into √P and vice versa
/// Ratio stands for P = token1/token0, not 1/P.
/// Uniswap uses ambigous terminology, sqrt_ratio means the same as sqrt_price
/// tick i: i24 ε [-429772, 429772]
/// sqrt ratio √P: f32 ε [-4.65 x 10^-10, 2.14 x 10^9]
/// f32 is used for √P and L, so we get x and y in f64 without overflow
/// Rust supports float so we can avoid Q64.96 format and assembly
/// Ref- https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/TickMath.sol

use ux::i24;

pub const MIN_TICK: i24 = -429772;
pub const MAX_TICK: i24 = -MIN_TICK;

pub const MIN_SQRT_RATIO: f64 = 0.0000000004656784964840615;
pub const MAX_SQRT_RATIO: f64 = 2147404287.6150422;

// √P = 1.0001^(i/2)
// Formula 6.2
pub fn get_sqrt_ratio_at_tick(tick: i24) -> f32 {
    todo!()
}

// i = floor(log_to_base(√1.0001) √P)
// Formula 6.2
pub fn get_tick_at_sqrt_ratio(sqrt_price: f32) -> i24 {
    todo!()
}

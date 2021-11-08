/// Convert tick i into √P and vice versa
/// Ratio stands for P = token1/token0, not 1/P.
/// Uniswap uses ambigous terminology, sqrt_ratio means the same as sqrt_price
/// tick i: i24 ε [-429772, 429772]
/// sqrt ratio √P: f32 ε [-4.65 x 10^-10, 2.14 x 10^9]
/// f32 is used for √P and L, so we get x and y in f64 without overflow
/// Rust supports float so we can avoid Q64.96 format and assembly
/// Ref- https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/TickMath.sol

// Need to recheck the calculation here.
pub const MIN_TICK: i32 = -429772;
pub const MAX_TICK: i32 = 429772;

pub const MIN_SQRT_RATIO: f64 = 0.0000000004656784964840615;
pub const MAX_SQRT_RATIO: f64 = 2147404287.6150422;

// √1.001
pub const BASE: f64 = 1.0000499987500623966241164453094825148582458496093750000000000000;

// √P = 1.0001^(i/2)
// Formula 6.2
pub fn get_sqrt_price_at_tick(tick: i32) -> f64 {   
    BASE.powi(tick)
}

// i = floor(log_to_base(√1.0001) √P)
// Formula 6.2
pub fn get_tick_at_sqrt_price(sqrt_price: f64) -> i32 {
    sqrt_price.log(BASE) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_from_sqrt_price() {
        println!("sqrt 1.0001: {:.64}", BASE.sqrt());

        let res = get_tick_at_sqrt_price(4.0);
        println!("got tick {} ", res);
    }

    #[test]
    fn test_sqrt_price_at_tick() {
        let res = get_sqrt_price_at_tick(10);
        println!("got sqrt price {} ", res);
    }
}

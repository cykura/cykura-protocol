///! Liquidity amount functions
///! Provides functions for computing liquidity amounts from token amounts and prices
///! Implements formula 6.29 and 6.30
///
use muldiv::MulDiv;
use super::fixed_point_x32;

/// Computes the amount of liquidity received for a given amount of token_0 and price range
/// Calculates ΔL = Δx (√P_upper x √P_lower)/(√P_upper - √P_lower)
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `amount_0` - The amount_0 being sent in
///
pub fn get_liquidity_for_amount_0(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    amount_0: u64,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };
    let intermediate = sqrt_ratio_a_x32
        .mul_div_floor(sqrt_ratio_b_x32, fixed_point_x32::Q32)
        .unwrap();

    amount_0
        .mul_div_floor(intermediate, sqrt_ratio_b_x32 - sqrt_ratio_a_x32)
        .unwrap()
}

/// Computes the amount of liquidity received for a given amount of token_1 and price range
/// Calculates ΔL = Δy / (√P_upper - √P_lower)
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `amount_1` - The amount_1 being sent in
///
pub fn get_liquidity_for_amount_1(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    amount_1: u64,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    amount_1
        .mul_div_floor(fixed_point_x32::Q32, sqrt_ratio_b_x32 - sqrt_ratio_a_x32)
        .unwrap()
}

/// Computes the maximum amount of liquidity received for a given amount of token_0, token_1, the current
/// pool prices and the prices at the tick boundaries
///
/// # Arguments
///
/// * `sqrt_ratio_x32` - A sqrt price representing the current pool prices
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `amount_0` - The amount of token_0 being sent in
/// * `amount_1` - The amount of token_1 being sent in
///
pub fn get_liquidity_for_amounts(
    sqrt_ratio_x32: u64,
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    amount_0: u64,
    amount_1: u64,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    if sqrt_ratio_x32 <= sqrt_ratio_a_x32 {
        // If P ≤ P_lower, only token_0 liquidity is active
        get_liquidity_for_amount_0(sqrt_ratio_a_x32, sqrt_ratio_b_x32, amount_0)
    } else if sqrt_ratio_x32 < sqrt_ratio_b_x32 {
        // If P_lower < P < P_upper, active liquidity is the minimum of the liquidity provided
        // by token_0 and token_1
        u64::min(
            get_liquidity_for_amount_0(sqrt_ratio_x32, sqrt_ratio_b_x32, amount_0),
            get_liquidity_for_amount_1(sqrt_ratio_a_x32, sqrt_ratio_x32, amount_1),
        )
    } else {
        // If P ≥ P_upper, only token_1 liquidity is active
        get_liquidity_for_amount_1(sqrt_ratio_a_x32, sqrt_ratio_b_x32, amount_1)
    }
}

/// Computes the amount of token_0 for a given amount of liquidity and a price range
/// Calculates Δx = ΔL (√P_upper - √P_lower) / (√P_upper x √P_lower)
///     = ΔL (1 / √P_lower -1 / √P_upper)
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `liquidity` - The liquidity being valued
///
pub fn get_amount_0_for_liquidity(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    liquidity: u64,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    // Token amount can't exceed u64
    (((liquidity as u128) << fixed_point_x32::RESOLUTION)
        .mul_div_floor(
            (sqrt_ratio_b_x32 - sqrt_ratio_a_x32) as u128,
            sqrt_ratio_b_x32 as u128,
        )
        .unwrap()
        / (sqrt_ratio_a_x32 as u128)) as u64
}

/// Computes the amount of token_1 for a given amount of liquidity and a price range
/// Calculates Δy = ΔL * (√P_upper - √P_lower)
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `liquidity` - The liquidity being valued
///
pub fn get_amount_1_for_liquidity(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    liquidity: u64,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    liquidity
        .mul_div_floor(sqrt_ratio_b_x32 - sqrt_ratio_a_x32, fixed_point_x32::Q32)
        .unwrap()
}

/// Computes the token_0 and token_1 value for a given amount of liquidity, the current
/// pool prices and the prices at the tick boundaries
///
/// # Arguments
///
/// * `sqrt_ratio_x32` - A sqrt price representing the current pool prices
/// * `sqrt_ratio_a_x32` - A sqrt price representing the first tick boundary
/// * `sqrt_ratio_b_x32` - A sqrt price representing the second tick boundary
/// * `liquidity` - The liquidity being valued
/// * `amount_0` - The amount of token_0
/// * `amount_1` - The amount of token_1
///
pub fn get_amounts_for_liquidity(
    sqrt_ratio_x32: u64,
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    liquidity: u64,
) -> (u64, u64) {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    if sqrt_ratio_x32 <= sqrt_ratio_a_x32 {
        // If P ≤ P_lower, active liquidity is entirely in token_0
        (
            get_amount_0_for_liquidity(sqrt_ratio_a_x32, sqrt_ratio_b_x32, liquidity),
            0,
        )
    } else if sqrt_ratio_x32 < sqrt_ratio_b_x32 {
        // If P_lower < P < P_upper, active liquidity is in token_0 and token_1
        (
            get_amount_0_for_liquidity(sqrt_ratio_x32, sqrt_ratio_b_x32, liquidity),
            get_amount_1_for_liquidity(sqrt_ratio_a_x32, sqrt_ratio_x32, liquidity),
        )
    } else {
        // If P ≥ P_upper, active liquidity is entirely in token_1
        (
            0,
            get_amount_1_for_liquidity(sqrt_ratio_a_x32, sqrt_ratio_b_x32, liquidity),
        )
    }
}

// TODO tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_amounts() {
        let gg = get_amounts_for_liquidity(4294967296, 4337916969, 4380866642, u32::MAX as u64);
        println!("{:?}", gg);
    }
}

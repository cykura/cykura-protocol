/// Helper functions to find price changes for change in token
/// supply and vice versa
extern crate muldiv;
use super::{fixed_point_x32, unsafe_math};
use muldiv::MulDiv;

/// Get sqrt current price from reserves of token_1 and token_0
///
/// Where token_0 is base (ETH) and token_1 is quote (USDC)
///
/// # Formula
/// `P = reserve_1 / reserve_0`
///
pub fn encode_price_sqrt_x32(reserve_1: u64, reserve_0: u64) -> u64 {
    ((reserve_1 as f64 / reserve_0 as f64).sqrt() * u64::pow(2, 32) as f64).round() as u64
}

/// Gets the next sqrt price √P' given a delta of token_0
///
/// Always round up because
/// 1. In the exact output case, token 0 supply decreases leading to price increase.
/// Move price up so that exact output is met.
/// 2. In the exact input case, token 0 supply increases leading to price decrease.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not guarantee exact output.
///
/// Use function for exact input or exact output swaps for token 0
///
/// # Formula
///
/// * `√P' = √P * L / (L + Δx * √P)`
/// * If Δx * √P overflows, use alternate form `√P' = L / (L/√P + Δx)`
///
/// # Proof
///
/// For constant y,
/// √P * L = y
/// √P' * L' = √P * L
/// √P' = √P * L / L'
/// √P' = √P * L / L'
/// √P' = √P * L / (L + Δx*√P)
///
/// # Arguments
///
/// * `sqrt_p_x32` - The starting price `√P`, i.e., before accounting for the token_1 delta,
/// where P is `token_1_supply / token_0_supply`
/// * `liquidity` - The amount of usable liquidity L
/// * `amount` - Delta of token 0 (Δx) to add or remove from virtual reserves
/// * `add` - Whether to add or remove the amount of token_0
///
pub fn get_next_sqrt_price_from_amount_0_rounding_up(
    sqrt_p_x32: u64,
    liquidity: u32,
    amount: u64,
    add: bool,
) -> u64 {
    // we short circuit amount == 0 because the result is otherwise not
    // guaranteed to equal the input price
    if amount == 0 {
        return sqrt_p_x32;
    };
    let numerator_1 = (liquidity as u64) << fixed_point_x32::RESOLUTION; // U32.32

    if add {
        // Used native overflow check instead of the `a * b / b == a` Solidity method
        // https://stackoverflow.com/q/70143451/7721443

        if let Some(product) = amount.checked_mul(sqrt_p_x32) {
            let denominator = numerator_1 + product;
            if denominator >= numerator_1 {
                return numerator_1.mul_div_ceil(sqrt_p_x32, denominator).unwrap();
            };
        }
        // Alternate form if overflow - `√P' = L / (L/√P + Δx)`
        unsafe_math::div_rounding_up(
            numerator_1,
            (numerator_1 / sqrt_p_x32).checked_add(amount).unwrap(),
        )
    } else {
        // if the product overflows, we know the denominator underflows
        // in addition, we must check that the denominator does not underflow
        // assert!(product / amount == sqrt_p_x32 && numerator_1 > product);
        let product = amount.checked_mul(sqrt_p_x32).unwrap();
        assert!(numerator_1 > product);

        let denominator = numerator_1 - product;
        numerator_1.mul_div_ceil(sqrt_p_x32, denominator).unwrap()
    }
}

/// Gets the next sqrt price given a delta of token_1
///
/// Always round down because
/// 1. In the exact output case, token 1 supply decreases leading to price decrease.
/// Move price down by rounding down so that exact output of token 0 is met.
/// 2. In the exact input case, token 1 supply increases leading to price increase.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not gurantee exact output for token 0.
///
///
/// # Formula
///
/// * `√P' = √P + Δy / L`
///
/// # Arguments
///
/// * `sqrt_p_x32` - The starting price `√P`, i.e., before accounting for the token_1 delta
/// * `liquidity` - The amount of usable liquidity L
/// * `amount` - Delta of token 1 (Δy) to add or remove from virtual reserves
/// * `add` - Whether to add or remove the amount of token_1
///
pub fn get_next_sqrt_price_from_amount_1_rounding_down(
    sqrt_p_x32: u64,
    liquidity: u32,
    amount: u64,
    add: bool,
) -> u64 {
    // if we are adding (subtracting), rounding down requires rounding the quotient down (up)
    // in both cases, avoid a mul_div for most inputs to save gas
    // if amount <= u32::MAX, overflows do not happen
    if add {
        // quotient - `Δy / L` as U32.32
        let quotient = if amount <= (u32::MAX as u64) {
            // u32::MAX or below so that amount x 2^32 does not overflow
            (amount << fixed_point_x32::RESOLUTION) / (liquidity as u64)
        } else {
            amount
                .mul_div_floor(fixed_point_x32::Q32, liquidity as u64)
                .unwrap()
        };

        sqrt_p_x32.checked_add(quotient).unwrap()
    } else {
        let quotient = if amount <= (u32::MAX as u64) {
            unsafe_math::div_rounding_up(amount << fixed_point_x32::RESOLUTION, liquidity as u64)
        } else {
            amount
                .mul_div_ceil(fixed_point_x32::Q32, liquidity as u64)
                .unwrap()
        };

        assert!(sqrt_p_x32 > quotient);
        sqrt_p_x32 - quotient
    }
}

/// Gets the next sqrt price given an input amount of token0 or token1
/// Throws if price or liquidity are 0, or if the next price is out of bounds
///
/// # Arguments
///
/// * `sqrt_p_x32` - The starting price `√P`, i.e., before accounting for the input amount
/// * `liquidity` - The amount of usable liquidity
/// * `amount_in` - How much of token_0, or token_1, is being swapped in
/// * `zero_for_one` - Whether the amount in is token_0 or token_1
///
pub fn get_next_sqrt_price_from_input(
    sqrt_p_x32: u64,
    liquidity: u32,
    amount_in: u64,
    zero_for_one: bool,
) -> u64 {
    assert!(sqrt_p_x32 > 0);
    assert!(liquidity > 0);

    // round to make sure that we don't pass the target price
    if zero_for_one {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_p_x32, liquidity, amount_in, true)
    } else {
        get_next_sqrt_price_from_amount_1_rounding_down(sqrt_p_x32, liquidity, amount_in, true)
    }
}

/// Gets the next sqrt price given an output amount of token0 or token1
///
/// Throws if price or liquidity are 0 or the next price is out of bounds
///
/// # Arguments
///
/// * `sqrt_p_x32` - The starting price `√P`, i.e., before accounting for the output amount
/// * `liquidity` - The amount of usable liquidity
/// * `amount_out` - How much of token_0, or token_1, is being swapped out
/// * `zero_for_one` - Whether the amount out is token_0 or token_1
///
pub fn get_next_sqrt_price_from_output(
    sqrt_p_x32: u64,
    liquidity: u32,
    amount_out: u64,
    zero_for_one: bool,
) -> u64 {
    assert!(sqrt_p_x32 > 0);
    assert!(liquidity > 0);

    if zero_for_one {
        get_next_sqrt_price_from_amount_1_rounding_down(sqrt_p_x32, liquidity, amount_out, false)
    } else {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_p_x32, liquidity, amount_out, false)
    }
}

/// Gets the amount_0 delta between two prices, for given amount of liquidity (formula 6.30)
///
/// # Formula
///
/// * `Δx = L * (1 / √P_lower - 1 / √P_upper)`
/// * i.e. `L * (√P_upper - √P_lower) / (√P_upper * √P_lower)`
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price
/// * `sqrt_ratio_b_x32` - Another sqrt price
/// * `liquidity` - The amount of usable liquidity
/// * `round_up`- Whether to round the amount up or down
///
pub fn get_amount_0_delta_unsigned(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    liquidity: u32,
    round_up: bool,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    let numerator_1 = (liquidity as u64) << fixed_point_x32::RESOLUTION;
    let numerator_2 = sqrt_ratio_b_x32 - sqrt_ratio_a_x32;

    assert!(sqrt_ratio_a_x32 > 0);

    if round_up {
        unsafe_math::div_rounding_up(
            numerator_1
                .mul_div_ceil(numerator_2, sqrt_ratio_b_x32)
                .unwrap(),
            sqrt_ratio_a_x32,
        )
    } else {
        numerator_1
            .mul_div_floor(numerator_2, sqrt_ratio_b_x32)
            .unwrap()
            / sqrt_ratio_a_x32
    }
}

/// Gets the amount_1 delta between two prices, for given amount of liquidity (formula 6.30)
///
/// # Formula
///
/// * `Δy = L (√P_upper - √P_lower)`
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price
/// * `sqrt_ratio_b_x32` - Another sqrt price
/// * `liquidity` - The amount of usable liquidity
/// * `round_up`- Whether to round the amount up or down
///
pub fn get_amount_1_delta_unsigned(
    mut sqrt_ratio_a_x32: u64,
    mut sqrt_ratio_b_x32: u64,
    liquidity: u32,
    round_up: bool,
) -> u64 {
    // sqrt_ratio_a_x32 should hold the smaller value
    if sqrt_ratio_a_x32 > sqrt_ratio_b_x32 {
        std::mem::swap(&mut sqrt_ratio_a_x32, &mut sqrt_ratio_b_x32);
    };

    if round_up {
        (liquidity as u64).mul_div_ceil(sqrt_ratio_b_x32 - sqrt_ratio_a_x32, fixed_point_x32::Q32)
    } else {
        (liquidity as u64).mul_div_floor(sqrt_ratio_b_x32 - sqrt_ratio_a_x32, fixed_point_x32::Q32)
    }
    .unwrap()
}

/// Helper function to get signed token_0 delta between two prices,
/// for the given change in liquidity
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price
/// * `sqrt_ratio_b_x32` - Another sqrt price
/// * `liquidity` - The change in liquidity for which to compute amount_0 delta
///
pub fn get_amount_0_delta_signed(
    sqrt_ratio_a_x32: u64,
    sqrt_ratio_b_x32: u64,
    liquidity: i32,
) -> i64 {
    if liquidity < 0 {
        -(get_amount_0_delta_unsigned(
            sqrt_ratio_a_x32,
            sqrt_ratio_b_x32,
            liquidity.abs() as u32,
            false,
        ) as i64)
    } else {
        // TODO check overflow, since i64::MAX < u64::MAX
        get_amount_0_delta_unsigned(sqrt_ratio_a_x32, sqrt_ratio_b_x32, liquidity as u32, true)
            as i64
    }
}

/// Helper function to get signed token_1 delta between two prices,
/// for the given change in liquidity
///
/// # Arguments
///
/// * `sqrt_ratio_a_x32` - A sqrt price
/// * `sqrt_ratio_b_x32` - Another sqrt price
/// * `liquidity` - The change in liquidity for which to compute amount_1 delta
///
pub fn get_amount_1_delta_signed(
    sqrt_ratio_a_x32: u64,
    sqrt_ratio_b_x32: u64,
    liquidity: i32,
) -> i64 {
    if liquidity < 0 {
        -(get_amount_1_delta_unsigned(
            sqrt_ratio_a_x32,
            sqrt_ratio_b_x32,
            liquidity.abs() as u32,
            false,
        ) as i64)
    } else {
        // TODO check overflow, since i64::MAX < u64::MAX
        get_amount_1_delta_unsigned(sqrt_ratio_a_x32, sqrt_ratio_b_x32, liquidity as u32, true)
            as i64
    }
}

#[cfg(test)]
mod sqrt_math {
    use super::*;
    // #getNextSqrtPriceFromInput
    mod sqrt_price_from_input {
        use super::*;

        #[test]
        #[should_panic]
        fn fails_if_price_is_zero() {
            get_next_sqrt_price_from_input(0, 0, u64::pow(10, 17), false);
        }

        #[test]
        #[should_panic]
        fn fails_if_liquidity_is_zero() {
            get_next_sqrt_price_from_input(1, 0, u64::pow(10, 17), true);
        }

        #[test]
        #[should_panic]
        fn fails_if_input_amount_overflows_the_price() {
            let sqrt_p_x32 = u64::MAX;
            let liquidity: u32 = 1024;
            let amount_in: u64 = 1024;

            // sqrt_p_x32.checked_add() should fail
            get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, amount_in, false);
        }

        #[test]
        fn any_input_amount_cannot_underflow_the_price() {
            let sqrt_p_x32 = 1;
            let liquidity = 1;
            let amount_in = u64::pow(2, 63);

            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, amount_in, true),
                1
            );
        }

        #[test]
        fn returns_input_price_if_amount_in_is_zero_and_zero_for_one_is_true() {
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, u32::pow(10, 7), 0, true),
                sqrt_p_x32
            );
        }

        #[test]
        fn returns_input_price_if_amount_in_is_zero_and_zero_for_one_is_false() {
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, u32::pow(10, 7), 0, false),
                sqrt_p_x32
            );
        }

        #[test]
        fn returns_the_minimum_price_for_max_inputs() {
            let sqrt_p_x32 = u64::MAX - 1;
            let liquidity = u32::MAX;
            let max_amount_no_overflow =
                u64::MAX - (((liquidity as u64) << fixed_point_x32::RESOLUTION) / sqrt_p_x32);

            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, max_amount_no_overflow, true),
                1
            );
        }

        #[test]
        fn input_amount_of_01_token_1() {
            // price of token 0 wrt token 1 increases as token_1 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            let liquidity = u32::pow(10, 8);
            let amount_0_in = u64::pow(10, 7); // 10^7 / 10^8 = 0.1
            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, amount_0_in, false),
                4724464025 // `√P' = √P + Δy / L`, rounded down
                           // https://www.wolframalpha.com/input/?i=floor%282%5E32+*+%281+%2B+0.1%29%29
            );
        }

        #[test]
        fn input_amount_of_01_token_0() {
            // price of token_0 wrt token_1 decreases as token_0 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            let liquidity = u32::pow(10, 8);
            let amount_0_in = u64::pow(10, 7); // 10^7 / 10^8 = 0.1
            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, amount_0_in, true),
                3904515724 // `√P' = √P * L / (L + Δx * √P)`, rounded up
                           // https://www.wolframalpha.com/input/?i=ceil%282%5E32+*+%281+%2F+%281+%2B+0.1%29%29%29
            );
        }

        #[test]
        fn amount_in_gt_u32_max_and_for_token_1() {
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            let liquidity = u32::pow(10, 8);
            let amount_0_in = u64::pow(10, 12); // 10^12 / 10^8 = 10^4
            assert_eq!(
                get_next_sqrt_price_from_input(sqrt_p_x32, liquidity, amount_0_in, true),
                429454 // `√P' = √P * L / (L + Δx * √P)`, rounded up
                       // https://www.wolframalpha.com/input/?i=ceil%282%5E32+*+%281+%2F+%281+%2B+10%5E4%29%29%29
            );
        }
    }

    // #getNextSqrtPriceFromOutput
    mod sqrt_price_from_output {
        use super::*;

        #[test]
        #[should_panic]
        fn fails_if_price_is_zero() {
            get_next_sqrt_price_from_output(0, 0, u64::pow(10, 17), false);
        }
        #[test]
        #[should_panic]
        fn fails_if_liquidity_is_zero() {
            get_next_sqrt_price_from_output(1, 0, u64::pow(10, 17), true);
        }

        #[test]
        #[should_panic]
        fn fails_if_output_amount_exactly_virtual_reserves_of_token_0() {
            let sqrt_p_x32 = 1099511627776;
            let liquidity: u32 = 1024;
            let amount_out: u64 = 4;

            get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_out, false);
        }
        #[test]
        #[should_panic]
        fn fails_if_output_amount_gt_virtual_reserves_of_token_0() {
            let sqrt_p_x32 = 1099511627776;
            let liquidity: u32 = 1024;
            let amount_out: u64 = 5;

            get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_out, false);
        }
        #[test]
        #[should_panic]
        fn fails_if_output_amount_gt_virtual_reserves_of_token_1() {
            let sqrt_p_x32 = 1099511627776;
            let liquidity: u32 = 1024;
            let amount_out: u64 = 262145;

            get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_out, true);
        }
        #[test]
        #[should_panic]
        fn fails_if_output_amount_exactly_virtual_reserves_of_token_1() {
            let sqrt_p_x32 = 1099511627776;
            let liquidity: u32 = 1024;
            let amount_out: u64 = 262144;

            get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_out, true);
        }

        #[test]
        fn succeeds_if_output_amount_is_lt_the_virtual_reserves_of_token1() {
            let sqrt_p_x32 = 1099511627776;
            let liquidity: u32 = 1024;
            let amount_out: u64 = 262143;

            assert_eq!(
                get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_out, true),
                4194304
            );
        }
        #[test]
        #[should_panic]
        fn puzzling_echidna_test() {
            unimplemented!();
        }
        #[test]
        fn returns_output_price_if_amount_out_is_zero_and_zero_for_one_is_true() {
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            assert_eq!(
                get_next_sqrt_price_from_output(sqrt_p_x32, u32::pow(10, 7), 0, true),
                sqrt_p_x32
            );
        }
        #[test]
        fn returns_output_price_if_amount_out_is_zero_and_zero_for_one_is_false() {
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            assert_eq!(
                get_next_sqrt_price_from_output(sqrt_p_x32, u32::pow(10, 7), 0, false),
                sqrt_p_x32
            );
        }
        #[test]
        fn output_amount_of_01_token_1() {
            // price of token 0 wrt token 1 increases as token_1 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            let liquidity = u32::pow(10, 8);
            let amount_0_out = u64::pow(10, 7); // 10^7 / 10^8 = 0.1
            assert_eq!(
                get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_0_out, false),
                4772185885 // `√P' = √P * L / (L + Δx * √P)`, rounded up
                          // https://www.wolframalpha.com/input/?i=ceil%282%5E32+*+%281+%2F+%281+%2B+0.1%29%29%29
            );
        }
        #[test]
        fn output_amount_of_01_token_0() {
            // price of token_0 wrt token_1 decreases as token_0 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1); //4294967296
            let liquidity = u32::pow(10, 8);
            let amount_0_out = u64::pow(10, 7); // 10^7 / 10^8 = 0.1

            assert_eq!(
                get_next_sqrt_price_from_output(sqrt_p_x32, liquidity, amount_0_out, true),
                3865470566 // `√P' = √P + Δy / L`, rounded down
                           // https://www.wolframalpha.com/input/?i=floor%284294967296+%2B+%2810%5E7%2F10%5E8%29%29
            );
        }
        #[test]
        #[should_panic]
        fn reverts_if_amount_out_is_impossible_in_zero_for_one_direction() {
            // price of token 0 wrt token 1 increases as token_1 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            get_next_sqrt_price_from_output(sqrt_p_x32, 1, u64::MAX, true);
        }
        #[test]
        #[should_panic]
        fn reverts_if_amount_out_is_impossible_in_one_for_zero_direction() {
            // price of token 0 wrt token 1 increases as token_1 supply increases
            let sqrt_p_x32 = encode_price_sqrt_x32(1, 1);
            get_next_sqrt_price_from_output(sqrt_p_x32, 1, u64::MAX, false);
        }
    }

    // #getAmount0Delta
    mod amount_0_delta {
        use super::*;
        #[test]
        fn returns_0_if_liquidity_is_0() {
            assert_eq!(
                get_amount_0_delta_unsigned(
                    encode_price_sqrt_x32(1, 1),
                    encode_price_sqrt_x32(2, 1),
                    0,
                    true
                ),
                0
            )
        }
        #[test]
        fn returns_0_if_prices_are_equal() {
            assert_eq!(
                get_amount_0_delta_unsigned(
                    encode_price_sqrt_x32(1, 1),
                    encode_price_sqrt_x32(1, 1),
                    0,
                    true
                ),
                0
            )
        }
        #[test]
        fn returns_01_amount0_for_price_of_1_to_121() {
            // 0.1 .................  1 .... 1.21
            let amount_0 = get_amount_0_delta_unsigned(
                encode_price_sqrt_x32(1, 1),
                encode_price_sqrt_x32(121, 100),
                u32::pow(10, 8),
                true,
            );

            assert_eq!(amount_0, 9090910);

            let amount_0_rounded_down = get_amount_0_delta_unsigned(
                encode_price_sqrt_x32(1, 1),
                encode_price_sqrt_x32(121, 100),
                u32::pow(10, 8),
                false,
            );
            assert_eq!(amount_0_rounded_down, amount_0.checked_sub(1).unwrap())
        }
        #[test]
        fn works_for_prices_that_overflow() {
            let amount_0_up = get_amount_0_delta_unsigned(
                encode_price_sqrt_x32(u64::pow(2, 17), 1), //encodePriceSqrt(BigNumber.from(2).pow(90), 1)
                encode_price_sqrt_x32(u64::pow(2, 18), 1), //encodePriceSqrt(BigNumber.from(2).pow(96), 1)
                u32::pow(10, 8),
                true,
            );
            let amount_0_down = get_amount_0_delta_unsigned(
                encode_price_sqrt_x32(u64::pow(2, 17), 1), //encodePriceSqrt(BigNumber.from(2).pow(90), 1)
                encode_price_sqrt_x32(u64::pow(2, 18), 1), //encodePriceSqrt(BigNumber.from(2).pow(96), 1)
                u32::pow(10, 8),
                false,
            );

            assert_eq!(amount_0_up, amount_0_down.checked_add(1).unwrap())
        }
    }

    // #getAmount1Delta
    mod amount_1_delta {
        use super::*;
        #[test]
        fn returns_0_if_liquidity_is_0() {
            assert_eq!(
                get_amount_1_delta_unsigned(
                    encode_price_sqrt_x32(1, 1),
                    encode_price_sqrt_x32(2, 1),
                    0,
                    true
                ),
                0
            )
        }
        #[test]
        fn returns_0_if_prices_are_equal() {
            assert_eq!(
                get_amount_1_delta_unsigned(
                    encode_price_sqrt_x32(1, 1),
                    encode_price_sqrt_x32(1, 1),
                    0,
                    true
                ),
                0
            )
        }
        #[test]
        fn returns_01_amount1_for_price_of_1_to_121() {
            // 0.1 .................  1 .... 1.21
            let amount_1 = get_amount_1_delta_unsigned(
                encode_price_sqrt_x32(1, 1),
                encode_price_sqrt_x32(121, 100),
                u32::pow(10, 8),
                true,
            );

            assert_eq!(amount_1, 10000001);

            let amount_1_rounded_down = get_amount_1_delta_unsigned(
                encode_price_sqrt_x32(1, 1),
                encode_price_sqrt_x32(121, 100),
                u32::pow(10, 8),
                false,
            );
            assert_eq!(amount_1_rounded_down, amount_1.checked_sub(1).unwrap());
        }
    }

    // swap computation
    #[test]
    #[should_panic]
    fn swap_computation() {
        todo!();
    }
}

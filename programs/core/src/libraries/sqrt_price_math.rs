/// Helper functions to find price changes for change in token
/// supply and vice versa

/// Get new sqrt price when token 0 is added or removed
/// Use function for exact input or exact output swaps for token 0
///
/// Formula
/// √P' = √P * L / (L + Δx * √P)
///
/// Proof
/// For constant y,
/// √P * L = y
/// √P' * L' = √P * L
/// √P' = √P * L / L'
/// √P' = √P * L / L'
/// √P' = √P * L / (L + Δx*√P)
///
/// If Δx * √P overflows, use alternate form
/// √P' = L / (L/√P + Δx)
///
/// Always round up because
/// 1. In exact output case, token 0 supply decreases leading to price increase.
/// Move price up so that exact output is met.
/// 2. In exact input case, token 0 supply increases leading to price decrease.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not gurantee exact output.
/// Where P = y/x or token_1_supply/token_0_supply
///
/// Formula
/// @param sqrt_price Current √P
/// @param liquidity Current liquidity in pool L
/// @param amount Amount of token 0 to add or remove Δx
/// @param add True of adding token 0, false if removing
/// @return New price after adding or removing Δy
pub fn get_next_sqrt_price_from_amount_0_rounding_up(
    sqrt_price: f64,
    liquidity: u32,
    amount: u64,
    add: bool,
) -> f64 {
    if amount == 0 {
        return sqrt_price;
    };

    // But no rounding off done
    sqrt_price * (liquidity as f64)
        / ((liquidity as f64) + (amount as f64) * if add { sqrt_price } else { -sqrt_price })

    
}

/// Get new sqrt price when token 1 is added or removed
/// Use function for exact input or exact output swaps for token 1
///
/// Formula
/// √P' = √P + Δy / L
///
/// Always round down because
/// 1. In exact output case, token 1 supply decreases leading to price decrease.
/// Move price down by rounding down so that exact output of token 0 is met.
/// 2. In exact input case, token 1 supply increases leading to price increase.
/// Do not round down to minimize price impact. We only need to meet input
/// change and not gurantee exact output for token 0.
/// Where P = y/x or token_1_supply/token_0_supply
///
/// Formula
/// @param sqrt_price Current √P
/// @param liquidity Current liquidity in pool L
/// @param amount Amount of token 1 to add or remove Δy
/// @param add True of adding token 1, false if removing
/// @return New price after adding or removing Δy
pub fn get_next_sqrt_price_from_amount_1_rounding_down(
    sqrt_price: f64,
    liquidity: u32,
    amount: u64,
    add: bool,
) -> f64 {
    // But no rounding off done

    sqrt_price + if add { amount as f64 } else { -(amount as f64) } / (liquidity as f64)
}

/// Convenience function to wrap get_next_sqrt_price_from_amount_0_rounding_up()
/// and get_next_sqrt_price_from_amount_1_rounding_up() for positive amounts.
/// @param sqrt_price Current √P
/// @param liquidity Current liquidity in pool L
/// @param amount_in Amount of token to add
/// zero_for_one Direction of swap. If true, amount_in is in token 0 else in token 1
/// @return New price after adding amount_in
pub fn get_next_sqrt_price_from_input(
    sqrt_price: f64,
    liquidity: u32,
    amount_in: u64,
    zero_for_one: bool,
) -> f64 {
    assert!(sqrt_price > 0.0);
    assert!(liquidity > 0);

    if zero_for_one {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount_in, true)
    } else {
        get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount_in, true)
    }
}

/// Convenience function to wrap get_next_sqrt_price_from_amount_0_rounding_up()
/// and get_next_sqrt_price_from_amount_1_rounding_up() for negative amounts.
/// @param sqrt_price Current √P
/// @param liquidity Current liquidity in pool L
/// @param amount_in Amount of token to remove
/// zero_for_one Direction of swap. If true, amount_out is in token 0 else in token 1
/// @return New price after removing amount_out
pub fn get_next_sqrt_price_from_output(
    sqrt_price: f64,
    liquidity: u32,
    amount_out: u64,
    zero_for_one: bool,
) -> f64 {
    assert!(sqrt_price > 0.0);
    assert!(liquidity > 0);

    if zero_for_one {
        get_next_sqrt_price_from_amount_1_rounding_down(sqrt_price, liquidity, amount_out, false)
    } else {
        get_next_sqrt_price_from_amount_0_rounding_up(sqrt_price, liquidity, amount_out, false)
    }
}

/// Get amount 0 delta between two prices (formula 6.30)
///
/// Δx = L (1 / √P_lower - 1 / √P_upper)
///
/// Used in SwapMath
/// @param sqrt_price_a
/// @param sqrt_price_b
/// @param liquidity Current liquidity L
/// @param round_up Round up if true, else round down
/// @return Amount 0 delta
pub fn get_amount_0_delta_unsigned(
    sqrt_price_a: f64,
    sqrt_price_b: f64,
    liquidity: u32,
    round_up: bool,
) -> u64 {
    let (min_price, max_price) = if sqrt_price_a > sqrt_price_b {
        (sqrt_price_b, sqrt_price_a)
    } else {
        (sqrt_price_a, sqrt_price_b)
    };
    assert!(sqrt_price_a > 0.0);

    // what about round up?
    (liquidity as u64) * ((1.0 / min_price - 1.0 / max_price) as u64)
}

/// Get amount 1 delta between two prices
///
/// Δy = L (√P_upper - √P_lower)
///
/// @param sqrt_price_a
/// @param sqrt_price_b
/// @param liquidity Current liquidity L
/// @param round_up Round up if true, else round down
/// @return Amount 1 delta
pub fn get_amount_1_delta_unsigned(
    sqrt_price_a: f64,
    sqrt_price_b: f64,
    liquidity: u32,
    round_up: bool,
) -> u64 {
    let (min_price, max_price) = if sqrt_price_a > sqrt_price_b {
        (sqrt_price_b, sqrt_price_a)
    } else {
        (sqrt_price_a, sqrt_price_b)
    };
    // if sqrt_price_a > sqrt_price_b {
    //     let temp = sqrt_price_a;
    //     sqrt_price_a = sqrt_price_b;
    //     sqrt_price_b = temp;
    // }

    // what about round up?
    ((liquidity as f64) * (min_price - max_price)) as u64
}

/// Convenience overloaded function to get amount 0 delta
/// Round down if liquidity is negative, else round up
///
/// Δx = L (1 / √P_lower - 1 / √P_upper)
///
/// Used in Pool
/// @param sqrt_price_a
/// @param sqrt_price_b
/// @param liquidity Current liquidity L
/// @return Amount 0 delta
pub fn get_amount_0_delta_signed(sqrt_price_a: f64, sqrt_price_b: f64, liquidity: i32) -> i64 {
    if liquidity < 0 {
        -(get_amount_0_delta_unsigned(sqrt_price_a, sqrt_price_b, -liquidity as u32, false) as i64)
    } else {
        get_amount_0_delta_unsigned(sqrt_price_a, sqrt_price_b, liquidity as u32, true) as i64
    }
}

/// Convenience overloaded function to get amount 1 delta
/// Round down if liquidity is negative, else round up
///
/// Δy = L (√P_upper - √P_lower)
///
/// @param sqrt_price_a
/// @param sqrt_price_b
/// @param liquidity Current liquidity L
/// @param round_up Round up if true, else round down
/// @return Amount 1 delta
pub fn get_amount_1_delta_signed(sqrt_price_a: f64, sqrt_price_b: f64, liquidity: i32) -> i64 {
    if liquidity < 0 {
        -(get_amount_1_delta_unsigned(sqrt_price_a, sqrt_price_b, -liquidity as u32, false) as i64)
    } else {
        get_amount_1_delta_unsigned(sqrt_price_a, sqrt_price_b, liquidity as u32, true) as i64
    }
}

// Error Codes
// #[error]
// pub enum ErrorCode {
//     #[msg("SqrtPrice should be greater than 0")]
//     MinSqrtPrice,
//     #[msg("liquidity should be greater than 0")]
//     MinLiquidity,
// }

#[cfg(test)]
mod sqrt_math {
    use std::ops::{Add, Div, Sub};

    use super::*;

    #[test]
    fn sqrt_price_from_amount() {
        let res = get_next_sqrt_price_from_amount_0_rounding_up(0.0, 20, 4, true);
        println!("value {} ", res);
        // assert_eq!(2 + 2, 4);
    }

    #[test]
    #[should_panic]
    fn fails_if_price_is_zero() {
        let res = get_next_sqrt_price_from_input(0.0, 0, 1E18 as u64 / 100, false);
        println!("{}", res);
    }

    // Failing: f64 MAX addition does not overflow
    #[test]
    #[should_panic]
    fn fails_if_input_amount_overflows_the_price() {
        let price = f64::MAX;
        let liquidity: u32 = 1024;
        let amount_in: u64 = 1024;
        //     await expect(sqrtPriceMath.getNextSqrtPriceFromInput(price, liquidity, amountIn, false)).to.be.reverted
        println!("Before {}", price);
        println!("Overflown number {}", price.add(1_f64));
        let res = get_next_sqrt_price_from_input(price.add(1_f64), liquidity, amount_in, false);
        println!("{}", res);
    } 

    // Failing
    #[test]
    fn fails_if_input_amount_underflows_the_price() {
        let res =  get_next_sqrt_price_from_input(1.0, 1u32, 2_u64.pow(255), true);
        println!("{}", res);
        assert_eq!(
            get_next_sqrt_price_from_input(1.0, 1u32, 2_u64.pow(255), true),
            1f64
        );
    }

    // Failing
    #[test]
    fn input_price_if_amount_in_zero() {
        let price = 1f64;
        assert_eq!(
            get_next_sqrt_price_from_input(price, 1E18 as u32, 0, true),
            price
        );
        assert_eq!(
            get_next_sqrt_price_from_input(price, 1E18 as u32, 0, false),
            price
        );
    }

    #[test]
    fn min_price_for_max_inputs() {
        let sqrt_price = 2_f64.powf(160.0).sub(1_f64);
        let max_amount_no_overflow = u64::MAX
            .checked_sub(u64::MAX.div(sqrt_price as u64))
            .unwrap();
        assert_eq!(
            get_next_sqrt_price_from_input(sqrt_price, u32::MAX, max_amount_no_overflow, true),
            1f64
        )
    }

    #[test]
    fn enough_amount_in() {
        assert_eq!(
            get_next_sqrt_price_from_input(1_f64, 1, u64::MAX / 2, true),
            1_f64
        )
    }

    #[test]
    fn price_for_0_1tokens() {
        // Calculates prices for 0.1 tokens
        let price_0 = get_next_sqrt_price_from_input(1f64, 1E18 as u32, 1E17 as u64, false);
        let price_1 = get_next_sqrt_price_from_input(1f64, 1E18 as u32, 1E17 as u64, true);

        // Need to calcuate actual values
        todo!();
        // assert_eq!(price_0, );
        // assert_eq!(price_1, );
    }

    // NEED TO CHECK FOR CACULATIONS LATER.
    //   it('amountIn > type(uint96).max and zeroForOne = true', async () => {
    //     expect(
    //       await sqrtPriceMath.getNextSqrtPriceFromInput(
    //         encodePriceSqrt(1, 1),
    //         expandTo18Decimals(10),
    //         BigNumber.from(2).pow(100),
    //         true
    //       )
    //       // perfect answer:
    //       // https://www.wolframalpha.com/input/?i=624999999995069620+-+%28%281e19+*+1+%2F+%281e19+%2B+2%5E100+*+1%29%29+*+2%5E96%29
    //     ).to.eq('624999999995069620')
    //   })

    #[test]
    #[should_panic]
    fn fails_if_liquidity_is_zero() {
        let res = get_next_sqrt_price_from_input(1.0, 0, 1E18 as u64, true);
        println!("{}", res);
    }
}

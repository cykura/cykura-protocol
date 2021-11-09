// Helper library to find result of a swap within a single tick range

use crate::libraries::sqrt_price_math::{
    get_amount_0_delta_unsigned, 
    get_amount_1_delta_unsigned, 
    get_next_sqrt_price_from_input, 
    get_next_sqrt_price_from_output
};

/// Find result of a swap within a single tick range
///
/// @param sqrt_price_current Current price of pool
/// @param sqrt_price_target Target price which can't be exceeded.
/// Infer swap direction using (sqrt_price_target - sqrt_price_current)
/// @param liquidity Usable liquidity
/// @param amount_remaining Amount which remains to be swapped in our out
/// @param fee_pips LP fee share as hundredth of a bip (1/100 x 0.01% = 10^6). Divide by 10^6.
/// @returns sqrt_price_next Price after swapping, not to exceed sqrt_price_target
/// @returns amount_in Amount to be swapped in, either token 0 or 1 depending
/// swap direction
/// @returns amount_out Amount to be swapped out, either token 0 or 1 depending
/// swap direction
/// @returns fee_amount Amount of input to be taken as fee
pub fn compute_swap_step(
    sqrt_price_current: f64,
    sqrt_price_target: f64,
    liquidity: u32,
    amount_remaining: i64,
    fee_pips: u32,
) -> (f64, u64, u64, u64) {
    // If we swap token 0 (ETH) -> token 1 (USDC) price of token 0 goes down
    let zero_for_one = sqrt_price_current >= sqrt_price_target;
    let exact_in = amount_remaining >= 0;

    let (mut sqrt_price_next, mut amount_in, mut amount_out, mut fee_amount) =
        (0.0, 0 as u64, 0 as u64, 0 as u64);

    if exact_in {
        // amount_remaining is positive
        let amount_remaining_less_fee =
            (amount_remaining as f64 * (1E6 - (fee_pips as f64)) / 1E6) as u64;

        let amount_in = if zero_for_one {
            get_amount_0_delta_unsigned(sqrt_price_target, sqrt_price_current, liquidity, true)
        } else {
            get_amount_1_delta_unsigned(sqrt_price_target, sqrt_price_current, liquidity, true)
        };

        let sqrt_price_next = if amount_remaining_less_fee >= amount_in {
            // max price is hit
            sqrt_price_target
        } else {
            // max price is not reached, find intermediary price
            get_next_sqrt_price_from_input(
                sqrt_price_current,
                liquidity,
                amount_remaining_less_fee,
                zero_for_one,
            )
        };
    } else {
        // exact out case, amount_remaining is negative
        let amount_out = if zero_for_one {
            get_amount_1_delta_unsigned(sqrt_price_target, sqrt_price_current, liquidity, false)
        } else {
            get_amount_0_delta_unsigned(sqrt_price_current, sqrt_price_target, liquidity, false)
        };

        sqrt_price_next = if (amount_remaining.abs() as u64) >= amount_out {
            sqrt_price_target
        } else {
            get_next_sqrt_price_from_output(
                sqrt_price_current, 
                liquidity, 
                amount_remaining.abs() as u64, 
                zero_for_one
            )
        }
    }

    // did we reach max possible price for given ticks?
    let max: bool =  sqrt_price_target == sqrt_price_next;

    if zero_for_one {
        let amount_in = if max && exact_in {
            amount_in
        } else {
            get_amount_0_delta_unsigned(sqrt_price_next, sqrt_price_current, liquidity, true)
        };
        let amount_out = if max && !exact_in {
            amount_out
        } else {
            get_amount_1_delta_unsigned(sqrt_price_next, sqrt_price_current, liquidity, false)
        };
    } else {
        let amount_in = if max && exact_in {
            amount_in
        } else {
            get_amount_1_delta_unsigned(sqrt_price_current, sqrt_price_next, liquidity, true)
        };
        let amount_out = if max && !exact_in {
            amount_out
        } else {
            get_amount_0_delta_unsigned(sqrt_price_current, sqrt_price_next, liquidity, false)
        };
    }

    // For exact output swaps, output amount cannot be exceeded even if more tokens are attainable.
    // Cap the output amount to not exceed the remaining output amount
    if !exact_in  && amount_out > amount_remaining.abs() as u64 {
        amount_out = amount_remaining.abs() as u64;
    }

    fee_amount = if exact_in && sqrt_price_next != sqrt_price_target {
        // If swap completed within the price tick range, remainder input amount as fee
        amount_remaining.abs() as u64 - amount_in
    }else {
        // Else take pip percentage as fee
        (amount_in as f64 * (1E6 - (fee_pips as f64)) / 1E6) as u64
    };

    (sqrt_price_next, amount_in, amount_out, fee_amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_comupte_swap() {
        todo!()
    }
}

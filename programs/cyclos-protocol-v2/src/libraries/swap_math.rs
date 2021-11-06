// Helper library to find result of a swap within a single tick range

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
    amount_remaining: u64,
    fee_pips: u32
) -> (f64, u64, u64, u64) {

}
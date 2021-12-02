use std::ops::Sub;

/// Store owed liquidity, fee growth per unit liquidity fees per position
use anchor_lang::prelude::*;

/// addr: [token_0, token_1, fee, owner, tick_lower, tick_upper]
#[account]
#[derive(Default)]
pub struct PositionState {
    pub bump: u8,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: u32,
    pub owner: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,

    // Virtual liquidity in the position the last time it was touched
    pub liquidity: u32,
    pub fee_growth_inside_0_last_x32: u64,
    pub fee_growth_inside_1_last_x32: u64,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

impl PositionState {
    // No getter function: PositionState is a PDA of [token0, token1, fee, owner, tickLower, tickUpper]

    /// Credit liquidity change and fee growth to a position
    pub fn update(
        self: &mut Self,
        liquidity_delta: i32,
        fee_growth_inside_0_x32: u64,
        fee_growth_inside_1_x32: u64,
    ) {
        let liqudity_next = if liquidity_delta == 0 {
            // Poke- credit fees to a position without adding more liquidity
            assert!(self.liquidity > 0, "No pokes for 0 liquidity positions");
            self.liquidity
        } else {
            if liquidity_delta > 0 {
                self.liquidity.checked_add(liquidity_delta.abs() as u32).unwrap()
            }else {
                self.liquidity.checked_sub(liquidity_delta.abs() as u32).unwrap()
            }
        };

        // Calculate accumulated Fees
        let tokens_owed_0 = (self.liquidity as u64)
            .checked_mul(fee_growth_inside_0_x32.sub(self.fee_growth_inside_0_last_x32) as u64)
            .unwrap();

        let tokens_owed_1 = (self.liquidity as u64)
            .checked_mul(fee_growth_inside_1_x32.sub(self.fee_growth_inside_1_last_x32) as u64)
            .unwrap();

        // Update the position
        if liquidity_delta != 0 {
            self.liquidity = liqudity_next;
        }
        self.fee_growth_inside_0_last_x32 = fee_growth_inside_0_x32;
        self.fee_growth_inside_1_last_x32 = fee_growth_inside_1_x32;

        if tokens_owed_0 > 0 || tokens_owed_1 > 0 {
            // overflow is acceptable, have to withdraw before you hit type(u64).max fees
            self.tokens_owed_0 += tokens_owed_0;
            self.tokens_owed_1 += tokens_owed_1;
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn pool_udpate() {
        todo!();
    }
}

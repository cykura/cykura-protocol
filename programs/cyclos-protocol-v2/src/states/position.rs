/// Store owed liquidity, fee growth per unit liquidity fees per position
use anchor_lang::prelude::*;
use ux::i24;

/// addr: [token_0, token_1, fee, owner, tick_lower, tick_upper]
#[account]
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
    pub fee_growth_inside_0_last: f64,
    pub fee_growth_inside_1_last: f64,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

impl PositionState {
    // No getter function: PositionState is a PDA of [token0, token1, fee, owner, tickLower, tickUpper]

    /// Credit liquidity change and fee growth to a position
    pub fn update(
        self: &mut Self,
        liquidity_delta: i32,
        fee_growth_inside_0: f64,
        fee_growth_inside_1: f64,
    ) {
        let liqudity_next = if liquidity_delta == 0 {
            // Poke- credit fees to a position without adding more liquidity
            assert!(self.liquidity > 0, "No pokes for 0 liquidity positions");
            self.liquidity
        } else {
            self.liqudity.checked_add(liquidity_delta).unwrap()
        };

        // Calculate accumulated Fees
        let token_owed_0 = (self.liquidity as u64)
            .checked_mul(fee_growth_inside_0.checked_sub(self.fee_growth_inside_0_last))
            .unwrap();

        let token_owed_1 = (self.liquidity as u64)
            .checked_mul(fee_growth_inside_1.checked_sub(self.fee_growth_inside_1_last))
            .unwrap();

        // Update the position
        if (liquidity_delta != 0) {
            self.liquidity = liqudity_next;
        }
        self.fee_growth_inside_0_last = fee_growth_inside_0;
        self.fee_growth_inside_1_last = fee_growth_inside_1;

        if (token_owned_0 > 0 || token_owned_1 > 0) {
            // overflow is acceptable, have to withdraw before you hit type(u64).max fees
            self.token_owed_0 += token_owed_0;
            self.token_owed_1 += token_owed_1;
        }
    }

    /// Update position with given liquidity_delta
    /// Position liquidity and flipped state in bitmap is updated
    /// From Pools._update_position()
    pub fn update_position(self: &mut Self, liquidity_delta: u32, tick: i24) {
        todo!()
    }

    /// Update position with new liquidity, and find Δtoken0 and Δtoken1 required
    /// to produce this liquidity_delta
    /// mint() -> modify_position() -> update_position() -> update()
    ///
    /// TODO check what noDelegateCall does
    pub fn modify_position(self: &mut Self, liquidity_delta: u32) -> (i64, i64) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_udpate() {
        todo!();
    }
}

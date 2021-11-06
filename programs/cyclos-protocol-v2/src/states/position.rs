/// Store owed liquidity, fee growth per unit liquidity fees per position
use anchor_lang::prelude::*;

#[account]
pub struct PositionState {
    pub bump: u8,
    // Virtual liquidity in the position the last time it was touched
    pub liquidity: u32,
    pub fee_growth_inside_0_last: f64,
    pub fee_growth_inside_1_last: f64,
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
        todo!()
    }
}
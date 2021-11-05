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
    pub fn update(self: &mut Self) {

    }
}
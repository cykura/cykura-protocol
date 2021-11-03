use anchor_lang::prelude::*;

#[account]
pub struct PositionState {
    pub bump: u8,
    pub liquidity: u128,
    pub tick_lower: u128,
    pub tick_upper: u128,
}

impl PositionState {
    pub fn update(self: &mut Self) {

    }
}
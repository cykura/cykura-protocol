use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct PositionManagerState {
    pub bump: u8,
    pub core: Pubkey,
}

#[account]
#[derive(Default)]
pub struct NonFungiblePositionState {
    pub bump: u8,
    pub liquidity: u32,
    pub fee_growth_inside_0_x32: u64,
    pub fee_growth_inside_1_x32: u64,
    pub tokens_owed_0: u64,
    pub tokens_owed_1: u64,
}

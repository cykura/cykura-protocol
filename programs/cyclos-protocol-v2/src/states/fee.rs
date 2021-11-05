use anchor_lang::prelude::*;

// copied from Uniswap. Smaller value needed since we use smaller data types?
pub const TICK_SPACING_MAX: u16 = 16_384; // exclusive
pub const FEE_TIER_MAX: u32 = 1_000_000; // exclusive. Stands for 100%

#[account]
pub struct FeeState {
    pub bump: u8,
    pub fee: u32,
    pub tick_spacing: u16,
}

#[event]
pub struct FeeAmountEnabledEvent {
    #[index]
    pub fee: u32,
    #[index]
    pub tick_spacing: u16,
}

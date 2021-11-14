use crate::libraries::tick_math::{MAX_TICK, MIN_TICK};
use anchor_lang::prelude::*;

// addr: [token0, token1, fee]
#[account]
pub struct PoolState {
    pub bump: u8,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: u32,
    pub tick_spacing: u16,

    pub liquidity: u32,
    pub sqrt_price: f64,

    // tick is a i32 in range [-429772, 429772]
    // Tick variable stores an arbitrary tick, not necessarily a multiple of tick_spacing
    pub tick: i32,

    pub fee_growth_global_0: f64,
    pub fee_growth_global_1: f64,

    pub fee_protocol: u8, // Leftmost 4 bits: fee_token_0, rightmost 4 bits: fee_token_1
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub unlocked: bool,
}

impl PoolState {
    /// protocol_fee_0 is stored as rightmost 4 bits
    /// Divide and get remainder for rightmost bits as u8
    pub fn get_fee_protocol_0(&self) -> u8 {
        self.fee_protocol % 4
    }

    /// protocol_fee_1 is stored as leftmost 4 bits
    /// Right shift by 4 places to get leftmost bits as u8
    pub fn get_fee_protocol_1(&self) -> u8 {
        self.fee_protocol >> 4
    }
    /// Check if lower < upper and ticks are in range
    pub fn check_ticks(tick_lower: i32, tick_upper: i32) {
        assert!(tick_lower < tick_upper, "Tick lower shoule be less than ");
        assert!(
            tick_lower >= MIN_TICK,
            "Tick lower should ne greater than MIN_TICK"
        );
        assert!(
            tick_upper <= MAX_TICK,
            "Tick upper should be less than MAX tick"
        );
    }
}

#[event]
pub struct InitPoolEvent {
    #[index]
    pub pool_state: Pubkey,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: u32,
    pub sqrt_price: f64,
    pub tick: i32,
}

#[event]
pub struct SetFeeProtocolEvent {
    #[index]
    pub pool_state: Pubkey,
    pub fee_protocol_0_old: u8,
    pub fee_protocol_1_old: u8,
    pub fee_protocol_0: u8,
    pub fee_protocol_1: u8,
}

#[event]
pub struct CollectProtocolEvent {
    #[index]
    pub pool_state: Pubkey,
    pub amount_0: u64,
    pub amount_1: u64,
}

#[event]
pub struct MintEvent {
    #[index]
    pub pool_state: Pubkey,
    pub mint_creator: Pubkey,
    pub position_state: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: u32,
    pub amount_0: i64,
    pub amount_1: i64,
}

#[event]
pub struct CollectEvent {
    #[index]
    pub pool_state: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount_0: i64,
    pub amount_1: i64,
}

#[event]
pub struct BurnEvent {
    #[index]
    pub pool_state: Pubkey,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: i64,
    pub amount_0: i64,
    pub amount_1: i64,
}


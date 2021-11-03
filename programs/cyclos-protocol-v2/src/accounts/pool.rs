use anchor_lang::prelude::*;



#[account]
pub struct PoolState {
    pub bump: u8,
    pub liquidity: u64,
    pub sqrt_price: f64,
    pub tick: i16,
    pub protocol_fees_token_0: f64,
    pub protocol_fees_token_1: f64,
    pub fee_growth_global_0: f64,
    pub fee_growth_global_1: f64,
    pub locked: bool,
}

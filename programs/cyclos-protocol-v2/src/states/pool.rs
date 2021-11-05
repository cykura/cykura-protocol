use anchor_lang::prelude::*;
use ux::i24;


#[account]
pub struct PoolState {
    // TODO also store token 1, token 2 and fee tier
    pub bump: u8,
    pub liquidity: u32,
    pub sqrt_price: f64,
    // tick is a i24 in range [-429772, 429772]
    // TODO add Borsh support to uX crate. See how uX_serde added serde support
    pub tick_left_bits: u16,
    pub tick_right_bits: u8,

    pub fee_growth_global_0: f64,
    pub fee_growth_global_1: f64,

    // Total uncollected protocol fees
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub locked: bool,
}

impl PoolState {
    pub fn get_tick(&self) -> i24 {
        todo!("Calculate i24 tick from tick_left_bits and tick_right_bits");
    }
    pub fn set_tick(&mut self, tick: i24) {
        todo!("Calculate and set tick_left_bits and tick_right_bits");
    }
}

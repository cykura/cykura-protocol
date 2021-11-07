/// Store owed liquidity, fee growth per unit liquidity fees per position
use anchor_lang::prelude::*;
use ux::i24;

/// addr: [token0, token1, fee, owner, tick_lower, tick_upper]
/// TODO store these params in the PDA
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

    /// Update position with given liquidity_delta
    /// Position liquidity and flipped state in bitmap is updated
    /// From Pools._update_position()
    pub fn update_position(
        self: &mut Self,
        liquidity_delta: u32,
        tick: i24,
    ) {
        todo!()
    }

    /// Update position with new liquidity, and find Δtoken0 and Δtoken1 required
    /// to produce this liquidity_delta
    /// mint() -> modify_position() -> update_position() -> update()
    ///
    /// TODO check what noDelegateCall does
    pub fn modify_position(
        self: &mut Self,
        liquidity_delta: u32
    ) -> (i64, i64) {
        todo!()
    }
}
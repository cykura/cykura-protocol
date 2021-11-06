/// Store data for each valid tick
/// A tick is valid if it is a multiple of tick_spacing
/// Ref- https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/Tick.sol

use anchor_lang::prelude::*;
use ux::i24;

#[account]
pub struct TickState {
    // TODO save PDA params [token0, token1, fee, tick] in state
    pub bump: u8,
    pub liquidity_net: u64,
    pub liquidity_gross: u64,
    pub fee_growth_outside_0: f64,
    pub fee_growth_outside_1: f64,
    // 3 oracle variables skipped
}

impl TickState {
    // Get fee growth within the tick range for token 0 and token 1
    // Formulae 6.17, 6.18, 6.19
    // Refer to excalidraw explanation
    pub fn get_fee_growth_inside(
        &self,
        tick_upper: i24,
        tick_lower: i24,
        tick_current: i24,
        fee_growth_global_0: f64,
        fee_growth_global_1: f64,
    ) -> (f64, f64) {
        todo!()
    }

    // Update init state, liquidity, fee growth and oracle variables for a tick
    // Return true if tick was flipped
    pub fn update(
        &mut self,
        // tick variable skipped. Tick is used to get this PDA
        tick_current: i24,
        liquidity_delta: i64, // liquidity to be added or subtracted. If we move from left to right then add
        fee_growth_global_0: f64,
        fee_growth_global_1: f64,
        upper: bool, // to update a position's upper or lower tick
        max_liquidity: u64 // found from tick_spacing_to_max_liquidity_per_tick()
        // 3 oracle variables skipped
    ) -> bool {
        todo!()
    }

    // Delete this PDA's account data
    pub fn clear(&mut self) {
        todo!()
    }

    // Transition to this tick, update fee_growth_outside and return its net liquidity
    // Modification from uniswap: self is the tick to which we transition.
    pub fn cross(
        &mut self,
        fee_growth_global_0: f64,
        fee_growth_global_1: f64
    ) -> u64 {
        todo!()
    }


}

// Higher the tick distance (less legal ticks), more is the max liquidity per tick
// Divide u64::MAX by total count of ticks for given spacing
pub fn tick_spacing_to_max_liquidity_per_tick(tick_spacing: i24) -> u64 {
    todo!()
}
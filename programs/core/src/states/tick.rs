use std::cell::RefMut;

///! Contains functions for managing tick processes and relevant calculations
///

use anchor_lang::prelude::*;
use crate::libraries::tick_math::{MAX_TICK, MIN_TICK};

/// Account storing info for a price tick
///
/// PDA of `[token_0, token_1, fee, tick]`
///
#[account(zero_copy)]
#[derive(Default)]
pub struct TickState {
    /// Bump to identify PDA
    pub bump: u8,

    /// The price tick whose info is stored in the account
    pub tick: i32,

    /// The total position liquidity that references this tick
    pub liquidity_net: u32,

    /// Amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left)
    pub liquidity_gross: u32,

    /// Fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    /// only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub fee_growth_outside_0_x32: u64,
    pub fee_growth_outside_1_x32: u64,

    /// The cumulative tick value on the other side of the tick
    pub tick_cumulative_outside: i64,

    /// The seconds per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    /// only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub seconds_per_liquidity_outside_x32: u64,

    /// The seconds spent on the other side of the tick (relative to the current tick)
    /// only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub seconds_outside: u32,
}

impl TickState {

    /// Retrieves the all time fee growth data in token_0 and token_1, per unit of liquidity,
    /// inside a position's tick boundaries.
    ///
    /// Calculates `fr = fg - f_below(lower) - f_above(upper)`, formula 6.19
    ///
    /// # Arguments
    ///
    /// * `tick_lower` - The lower tick boundary of the position
    /// * `tick_upper` - The upper tick boundary of the position
    /// * `tick_current` - The current tick
    /// * `fee_growth_global_0_x32` - The all-time global fee growth, per unit of liquidity, in token_0
    /// * `fee_growth_global_1_x32` - The all-time global fee growth, per unit of liquidity, in token_1
    ///
    pub fn get_fee_growth_inside(
        tick_lower: &TickState,
        tick_upper: &TickState,
        tick_current: i32,
        fee_growth_global_0_x32: u64,
        fee_growth_global_1_x32: u64,
    ) -> (u64, u64) {
        // calculate fee growth below
        let (fee_growth_below_0_x32, fee_growth_below_1_x32) = if tick_current >= tick_lower.tick {(
            tick_lower.fee_growth_outside_0_x32,
            tick_lower.fee_growth_outside_1_x32,
        )} else {(
            fee_growth_global_0_x32 - tick_lower.fee_growth_outside_0_x32,
            fee_growth_global_1_x32 - tick_lower.fee_growth_outside_1_x32,
        )};

        // Calculate fee growth above
        let (fee_growth_above_0_x32, fee_growth_above_1_x32) = if tick_current < tick_upper.tick {(
            tick_upper.fee_growth_outside_0_x32,
            tick_upper.fee_growth_outside_1_x32,
        )} else {(
            fee_growth_global_0_x32 - tick_upper.fee_growth_outside_0_x32,
            fee_growth_global_1_x32 - tick_upper.fee_growth_outside_1_x32,
        )};
        let fee_growth_inside_0_x32 = fee_growth_global_0_x32 - fee_growth_below_0_x32 - fee_growth_above_0_x32;
        let fee_growth_inside_1_x32 = fee_growth_global_1_x32 - fee_growth_below_1_x32 - fee_growth_above_1_x32;

        (fee_growth_inside_0_x32, fee_growth_inside_1_x32)
    }

    // Update init state, liquidity, fee growth and oracle variables for a tick
    // Return true if tick was flipped
    // pub fn update(
    //     &mut self,
    //     // tick variable skipped. Tick is used to get this PDA
    //     tick_current: i32,
    //     liquidity_delta: i32, // liquidity to be added or subtracted. If we move from left to right then add
    //     fee_growth_global_0_x32: u64,
    //     fee_growth_global_1_x32: u64,
    //     upper: bool, // to update a position's upper or lower tick
    //     max_liquidity: u32, // found from tick_spacing_to_max_liquidity_per_tick()
    //     // 3 oracle variables skipped
    // ) -> bool {
    //     let liquidity_gross_before = self.liquidity_gross;

    //     let liquidity_gross_after = if liquidity_delta.is_positive() {
    //         liquidity_gross_before.checked_add(liquidity_delta as u32)
    //     } else {
    //         liquidity_gross_before.checked_sub(liquidity_delta.abs() as u32)
    //     }
    //     .unwrap();

    //     assert!(
    //         liquidity_gross_after <= max_liquidity,
    //         "Liquidity gross cannot exceed max liq"
    //     );

    //     // If liquidity was removed or added from a tick o
    //     // Either liquidity_gross_after becomes 0 (withdrawn) XOR liquidity_gross_before
    //     // was zero (liquidity added)
    //     let flipped = (liquidity_gross_after == 0) != (liquidity_gross_before == 0);

    //     if liquidity_gross_before == 0 {
    //         // if tick was just initialized (liquidity added), all fee growth happening
    //         // before initialization is taken to be below the tick
    //         if self.tick < tick_current {
    //             self.fee_growth_outside_0_x32 = fee_growth_global_0_x32;
    //             self.fee_growth_outside_1_x32 = fee_growth_global_1_x32;
    //             // Oracle variables skipped
    //         }
    //         self.initialized = true;
    //     }

    //     self.liquidity_gross = liquidity_gross_after;

    //     // when the lower (upper) tick is crossed left to right (right to left),
    //     // liquidity must be added (removed)
    //     self.liquidity_net = if upper {
    //         self.liquidity_net.checked_sub(liquidity_delta.abs() as u32)
    //     } else {
    //         self.liquidity_net.checked_add(liquidity_delta.abs() as u32)
    //     }
    //     .unwrap();

    //     flipped
    // }

    // Clear stored data
    // Delete account after clearing
    // TODO replace- clear() should de-initialize tick account
    // pub fn clear(&mut self) {
    //     self.bump = 0;
    //     self.token_0 = Pubkey::default();
    //     self.token_1 = Pubkey::default();
    //     self.fee = 0;
    //     self.tick = 0;
    //     self.liquidity_net = 0;
    //     self.liquidity_gross = 0;
    //     self.fee_growth_outside_0_x32 = 0;
    //     self.fee_growth_outside_1_x32 = 0;
    //     self.initialized = false;
    // }

    // Transition to this tick, update fee_growth_outside and return its net liquidity
    // Modification from uniswap: tick is the tick to which we transition.
    // pub fn cross(&mut self, fee_growth_global_0_x32: u64, fee_growth_global_1_x32: u64) -> u32 {
    //     self.fee_growth_outside_0_x32 = fee_growth_global_0_x32 - self.fee_growth_outside_0_x32;
    //     self.fee_growth_outside_1_x32 = fee_growth_global_1_x32 - self.fee_growth_outside_1_x32;
    //     // skip oracle variables
    //     self.liquidity_net
    // }
}

// Higher the tick distance (less legal ticks), more is the max liquidity per tick
// Divide u64::MAX by total count of ticks for given spacing
// pub fn tick_spacing_to_max_liquidity_per_tick(tick_spacing: i32) -> u32 {
//     let min_tick = (MIN_TICK / tick_spacing) * tick_spacing;
//     let max_tick = (MAX_TICK / tick_spacing) * tick_spacing;
//     let num_ticks = (max_tick - min_tick / tick_spacing) + 1;
//     let max_liquidity = u32::MAX;
//     max_liquidity / (num_ticks.abs() as u32)
// }

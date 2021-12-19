use crate::error::ErrorCode;
use crate::libraries::{liquidity_math, tick_math};
///! Contains functions for managing tick processes and relevant calculations
///!
use anchor_lang::prelude::*;

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
    pub liquidity_net: i64,

    /// Amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left)
    pub liquidity_gross: u64,

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
        let (fee_growth_below_0_x32, fee_growth_below_1_x32) = if tick_current >= tick_lower.tick {
            (
                tick_lower.fee_growth_outside_0_x32,
                tick_lower.fee_growth_outside_1_x32,
            )
        } else {
            (
                fee_growth_global_0_x32 - tick_lower.fee_growth_outside_0_x32,
                fee_growth_global_1_x32 - tick_lower.fee_growth_outside_1_x32,
            )
        };

        // Calculate fee growth above
        let (fee_growth_above_0_x32, fee_growth_above_1_x32) = if tick_current < tick_upper.tick {
            (
                tick_upper.fee_growth_outside_0_x32,
                tick_upper.fee_growth_outside_1_x32,
            )
        } else {
            (
                fee_growth_global_0_x32 - tick_upper.fee_growth_outside_0_x32,
                fee_growth_global_1_x32 - tick_upper.fee_growth_outside_1_x32,
            )
        };
        let fee_growth_inside_0_x32 =
            fee_growth_global_0_x32 - fee_growth_below_0_x32 - fee_growth_above_0_x32;
        let fee_growth_inside_1_x32 =
            fee_growth_global_1_x32 - fee_growth_below_1_x32 - fee_growth_above_1_x32;

        (fee_growth_inside_0_x32, fee_growth_inside_1_x32)
    }

    /// Updates a tick and returns true if the tick was flipped from initialized to uninitialized, or vice versa
    ///
    /// # Arguments
    ///
    /// * `self` - The tick state that will be updated
    /// * `tick_current` - The current tick
    /// * `liquidity_delta` - A new amount of liquidity to be added (subtracted) when tick is crossed
    /// from left to right (right to left)
    /// * `fee_growth_global_0_x32` - The all-time global fee growth, per unit of liquidity, in token_0
    /// * `fee_growth_global_1_x32` - The all-time global fee growth, per unit of liquidity, in token_1
    /// * `seconds_per_liquidity_cumulative_x32` - The all-time seconds per max(1, liquidity) of the pool
    /// * `tick_cumulative` - The tick * time elapsed since the pool was first initialized
    /// * `time` - The current block timestamp cast to a u32
    /// * `upper` - true for updating a position's upper tick, or false for updating a position's lower tick
    /// * `max_liquidity` - The maximum liquidity allocation for a single tick
    ///
    pub fn update(
        &mut self,
        tick_current: i32,
        liquidity_delta: i64,
        fee_growth_global_0_x32: u64,
        fee_growth_global_1_x32: u64,
        seconds_per_liquidity_cumulative_x32: u64,
        tick_cumulative: i64,
        time: u32,
        upper: bool,
        max_liquidity: u64,
    ) -> Result<bool, ProgramError> {
        let liquidity_gross_before = self.liquidity_gross;
        let liquidity_gross_after =
            liquidity_math::add_delta(liquidity_gross_before, liquidity_delta)?;

        require!(liquidity_gross_after <= max_liquidity, ErrorCode::LO);

        // Either liquidity_gross_after becomes 0 (uninitialized) XOR liquidity_gross_before
        // was zero (initialized)
        let flipped = (liquidity_gross_after == 0) != (liquidity_gross_before == 0);

        if liquidity_gross_before == 0 {
            // by convention, we assume that all growth before a tick was initialized happened _below_ the tick
            if self.tick < tick_current {
                self.fee_growth_outside_0_x32 = fee_growth_global_0_x32;
                self.fee_growth_outside_1_x32 = fee_growth_global_1_x32;
                self.seconds_per_liquidity_outside_x32 = seconds_per_liquidity_cumulative_x32;
                self.tick_cumulative_outside = tick_cumulative;
                self.seconds_outside = time;
            }
        }

        self.liquidity_gross = liquidity_gross_after;

        // when the lower (upper) tick is crossed left to right (right to left),
        // liquidity must be added (removed)
        self.liquidity_net = if upper {
            self.liquidity_net.checked_sub(liquidity_delta)
        } else {
            self.liquidity_net.checked_add(liquidity_delta)
        }
        .unwrap();

        Ok(flipped)
    }

    /// Transitions to the current tick as needed by price movement, returning the amount of liquidity
    /// added (subtracted) when tick is crossed from left to right (right to left)
    ///
    /// # Arguments
    ///
    /// * `self` - The destination tick of the transition
    /// * `fee_growth_global_0_x32` - The all-time global fee growth, per unit of liquidity, in token_0
    /// * `fee_growth_global_1_x32` - The all-time global fee growth, per unit of liquidity, in token_1
    /// * `seconds_per_liquidity_cumulative_x32` - The current seconds per liquidity
    /// * `tick_cumulative` - The tick * time elapsed since the pool was first initialized
    /// * `time` - The current block timestamp
    ///
    pub fn cross(
        &mut self,
        fee_growth_global_0_x32: u64,
        fee_growth_global_1_x32: u64,
        seconds_per_liquidity_cumulative_x32: u64,
        tick_cumulative: i64,
        time: u32,
    ) -> i64 {
        self.fee_growth_outside_0_x32 = fee_growth_global_0_x32 - self.fee_growth_outside_0_x32;
        self.fee_growth_outside_1_x32 = fee_growth_global_1_x32 - self.fee_growth_outside_1_x32;
        self.seconds_per_liquidity_outside_x32 =
            seconds_per_liquidity_cumulative_x32 - self.seconds_per_liquidity_outside_x32;
        self.tick_cumulative_outside = tick_cumulative - self.tick_cumulative_outside;
        self.seconds_outside = time - self.seconds_outside;

        self.liquidity_net
    }

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
}

/// Derives max liquidity per tick from given tick spacing
///
/// # Arguments
///
/// * `tick_spacing` - The amount of required tick separation, realized in multiples of `tick_sacing`
/// e.g., a tickSpacing of 3 requires ticks to be initialized every 3rd tick i.e., ..., -6, -3, 0, 3, 6, ...
///
pub fn tick_spacing_to_max_liquidity_per_tick(tick_spacing: i32) -> u64 {
    // Find min and max values permitted by tick spacing
    let min_tick = (tick_math::MIN_TICK / tick_spacing) * tick_spacing;
    let max_tick = (tick_math::MAX_TICK / tick_spacing) * tick_spacing;
    let num_ticks = ((max_tick - min_tick) / tick_spacing) as u64 + 1;

    u64::MAX / num_ticks
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn liquidity_per_tick() {
        let spacing = tick_spacing_to_max_liquidity_per_tick(200);
        msg!("{}", spacing);
    }
}

use crate::libraries::tick_math::{MAX_TICK, MIN_TICK};
/// Store data for each valid tick
/// A tick is valid if it is a multiple of tick_spacing
/// Ref- https://github.com/Uniswap/v3-core/blob/main/contracts/libraries/Tick.sol
use anchor_lang::prelude::*;

// addr: [token_0, token_1, fee, tick]
#[account]
#[derive(Default)]
pub struct TickState {
    pub bump: u8,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: u32,
    pub tick: i32,
    pub liquidity_net: u32,
    pub liquidity_gross: u32,
    pub fee_growth_outside_0: f64,
    pub fee_growth_outside_1: f64,
    // 3 oracle variables skipped

    // save gas by avoiding bitmap lookup
    pub initialized: bool,
}

impl TickState {
    /// Get fee growth within the tick range for token 0 and token 1
    /// fee inside = fee_growth_global - fee_growth_below_tick_lower - fee_growth_above_tick_upper
    /// Formulae 6.17, 6.18, 6.19
    /// Refer to excalidraw explanation
    pub fn get_fee_growth_inside(
        tick_upper: &Account<TickState>,
        tick_lower: &Account<TickState>,
        tick_current: i32,
        fee_growth_global_0: f64,
        fee_growth_global_1: f64,
    ) -> (f64, f64) {
        // calculate fee growth below
        let (fee_growth_below_0, fee_growth_below_1) = if tick_current >= tick_upper.tick {
            (
                tick_lower.fee_growth_outside_0,
                tick_lower.fee_growth_outside_1,
            )
        } else {
            (
                fee_growth_global_0 - tick_lower.fee_growth_outside_0,
                fee_growth_global_1 - tick_lower.fee_growth_outside_1,
            )
        };
        // Calculate fee growth above
        let (fee_growth_above_0, fee_growth_above_1) = if tick_current < tick_upper.tick {
            (
                tick_upper.fee_growth_outside_0,
                tick_upper.fee_growth_outside_1,
            )
        } else {
            (
                fee_growth_global_0 - tick_upper.fee_growth_outside_0,
                fee_growth_global_1 - tick_upper.fee_growth_outside_1,
            )
        };
        let fee_growth_inside_0 = fee_growth_global_0 - fee_growth_below_0 - fee_growth_above_0;
        let fee_growth_inside_1 = fee_growth_global_1 - fee_growth_below_1 - fee_growth_above_1;

        (fee_growth_inside_0, fee_growth_inside_1)
    }

    // Update init state, liquidity, fee growth and oracle variables for a tick
    // Return true if tick was flipped
    pub fn update(
        &mut self,
        // tick variable skipped. Tick is used to get this PDA
        tick_current: i32,
        liquidity_delta: i32, // liquidity to be added or subtracted. If we move from left to right then add
        fee_growth_global_0: f64,
        fee_growth_global_1: f64,
        upper: bool, // to update a position's upper or lower tick
        max_liquidity: u32, // found from tick_spacing_to_max_liquidity_per_tick()
        // 3 oracle variables skipped
    ) -> bool {
        let liquidity_gross_before = self.liquidity_gross;

        let liquidity_gross_after = if liquidity_delta.is_positive() {
            liquidity_gross_before.checked_add(liquidity_delta as u32)
        } else {
            liquidity_gross_before.checked_sub(liquidity_delta.abs() as u32)
        }
        .unwrap();

        assert!(
            liquidity_gross_after <= max_liquidity,
            "Liquidity gross cannot exceed max liq"
        );

        // If liquidity was removed or added from a tick o
        // Either liquidity_gross_after becomes 0 (withdrawn) XOR liquidity_gross_before
        // was zero (liquidity added)
        let flipped = (liquidity_gross_after == 0) != (liquidity_gross_before == 0);

        if liquidity_gross_before == 0 {
            // if tick was just initialized (liquidity added), all fee growth happening
            // before initialization is taken to be below the tick
            if self.tick < tick_current {
                self.fee_growth_outside_0 = fee_growth_global_0;
                self.fee_growth_outside_1 = fee_growth_global_1;
                // Oracle variables skipped
            }
            self.initialized = true;
        }

        self.liquidity_gross = liquidity_gross_after;

        // when the lower (upper) tick is crossed left to right (right to left),
        // liquidity must be added (removed)
        self.liquidity_net = if upper {
            self.liquidity_net.checked_sub(liquidity_delta.abs() as u32)
        } else {
            self.liquidity_net.checked_add(liquidity_delta.abs() as u32)
        }
        .unwrap();

        flipped
    }

    // Clear stored data
    // Delete account after clearing
    pub fn clear(&mut self) {
        self.bump = 0;
        self.token_0 = Pubkey::default();
        self.token_1 = Pubkey::default();
        self.fee = 0;
        self.tick = 0;
        self.liquidity_net = 0;
        self.liquidity_gross = 0;
        self.fee_growth_outside_0 = 0.0;
        self.fee_growth_outside_1 = 0.0;
        self.initialized = false;
    }

    // Transition to this tick, update fee_growth_outside and return its net liquidity
    // Modification from uniswap: tick is the tick to which we transition.
    pub fn cross(&mut self, fee_growth_global_0: f64, fee_growth_global_1: f64) -> u32 {
        self.fee_growth_outside_0 = fee_growth_global_0 - self.fee_growth_outside_0;
        self.fee_growth_outside_1 = fee_growth_global_1 - self.fee_growth_outside_1;
        // skip oracle variables
        self.liquidity_net
    }
}

// Higher the tick distance (less legal ticks), more is the max liquidity per tick
// Divide u64::MAX by total count of ticks for given spacing
pub fn tick_spacing_to_max_liquidity_per_tick(tick_spacing: i32) -> u32 {
    let min_tick = (MIN_TICK / tick_spacing) * tick_spacing;
    let max_tick = (MAX_TICK / tick_spacing) * tick_spacing;
    let num_ticks = (max_tick - min_tick / tick_spacing) + 1;
    let max_liquidity = u32::MAX;
    max_liquidity / (num_ticks.abs() as u32)
}

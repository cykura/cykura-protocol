///! Positions represent an owner address' liquidity between a lower and upper tick boundary
///! Positions store additional state for tracking fees owed to the position
///!

use anchor_lang::prelude::*;
use crate::{error::ErrorCode, libraries::{liquidity_math, fixed_point_x32}};
extern crate muldiv;
use muldiv::MulDiv;

/// Info stored for each user's position
///
/// PDA of `[POSITION_SEED, token_0, token_1, fee, owner, tick_lower, tick_upper]`
///
#[account(zero_copy)]
#[derive(Default)]
pub struct PositionState {
    /// Bump to identify PDA
    pub bump: u8,

    /// The amount of liquidity owned by this position
    pub liquidity: u64,

    /// The token_0 fee growth per unit of liquidity as of the last update to liquidity or fees owed
    pub fee_growth_inside_0_last_x32: u64,

    /// The token_1 fee growth per unit of liquidity as of the last update to liquidity or fees owed
    pub fee_growth_inside_1_last_x32: u64,

    /// The fees owed to the position owner in token_0
    pub tokens_owed_0: u64,

    /// The fees owed to the position owner in token_1
    pub tokens_owed_1: u64,
}

impl PositionState {
    /// Credits accumulated fees to a user's position
    ///
    /// # Arguments
    ///
    /// * `self` - The individual position to update
    /// * `liquidity_delta` - The change in pool liquidity as a result of the position update
    /// * `fee_growth_inside_0_x32` - The all-time fee growth in token_0, per unit of liquidity,
    /// inside the position's tick boundaries
    /// * `fee_growth_inside_1_x32` - The all-time fee growth in token_1, per unit of liquidity,
    /// inside the position's tick boundaries
    ///
    pub fn update(
        &mut self,
        liquidity_delta: i64,
        fee_growth_inside_0_x32: u64,
        fee_growth_inside_1_x32: u64,
    ) -> Result<(), ErrorCode> {
        let liquidity_next = if liquidity_delta == 0 {
            require!(self.liquidity > 0, ErrorCode::NP); // disallow pokes for 0 liquidity positions
            self.liquidity
        } else {
            liquidity_math::add_delta(self.liquidity, liquidity_delta)?
        };

        // calculate accumulated Fees
        let tokens_owed_0 = (fee_growth_inside_0_x32 - self.fee_growth_inside_0_last_x32)
            .mul_div_floor(
                self.liquidity as u64,
                fixed_point_x32::Q32
            ).unwrap();
        let tokens_owed_1 = (fee_growth_inside_1_x32 - self.fee_growth_inside_1_last_x32)
            .mul_div_floor(
                self.liquidity as u64,
                fixed_point_x32::Q32
            ).unwrap();

        // Update the position
        if liquidity_delta != 0 {
            self.liquidity = liquidity_next;
        }
        self.fee_growth_inside_0_last_x32 = fee_growth_inside_0_x32;
        self.fee_growth_inside_1_last_x32 = fee_growth_inside_1_x32;
        if tokens_owed_0 > 0 || tokens_owed_1 > 0 {
            // overflow is acceptable, have to withdraw before you hit u64::MAX fees
            self.tokens_owed_0 += tokens_owed_0;
            self.tokens_owed_1 += tokens_owed_1;
        }

        Ok(())
    }
}

/// Emitted when liquidity is minted for a given position
#[event]
pub struct MintEvent {
    /// The pool for which liquidity was minted
    #[index]
    pub pool_state: Pubkey,

    /// The address that minted the liquidity
    pub sender: Pubkey,

    /// The owner of the position and recipient of any minted liquidity
    pub owner: Pubkey,

    /// The lower tick of the position
    #[index]
    pub tick_lower: i32,

    /// The upper tick of the position
    #[index]
    pub tick_upper: i32,

    /// The amount of liquidity minted to the position range
    pub amount: u64,

    /// How much token_0 was required for the minted liquidity
    pub amount_0: u64,

    /// How much token_1 was required for the minted liquidity
    pub amount_1: u64,
}

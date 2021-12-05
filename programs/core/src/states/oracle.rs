/// Oracle provides price and liquidity data useful for a wide variety of system designs
///
/// Instances of stored oracle data, "observations", are collected in the oracle array,
/// represented as PDAs with array index as seed.
///
/// Every pool is initialized with an oracle array length of 1. Anyone can pay to increase the
/// max length of the array, by initializing new accounts. New slots will be added when the
/// array is fully populated.
///
/// Observations are overwritten when the full length of the oracle array is populated.
///
/// The most recent observation is available, independent of the length of the oracle array,
/// by passing 0 as the index seed.
///

/// Returns data about a specific observation index
///
/// PDA of `[token_0, token_1, fee, index]`
///
use anchor_lang::prelude::*;

#[account]
pub struct ObservationState {
    /// Bump to identify PDA
    pub bump: u8,

    /// The element of the observations array stored in this account
    pub index: u16,

    /// The block timestamp of the observation
    pub block_timestamp: u32,

    /// The tick multiplied by seconds elapsed for the life of the pool as of the observation timestamp
    pub tick_cumulative: u64,

    /// The seconds per in range liquidity for the life of the pool as of the observation timestamp
    pub seconds_per_liquidity_cumulative_x32: u64,

    /// Whether the observation has been initialized and the values are safe to use
    pub initialized: bool,
}

/// Returns the block timestamp truncated to 32 bits, i.e. mod 2**32
///
pub fn _block_timestamp() -> u32 {
    Clock::get().unwrap().unix_timestamp as u32 // truncation is desired
}

/// Emitted by the pool for increases to the number of observations that can be stored
///
/// `observation_cardinality_next` is not the observation cardinality until an observation
/// is written at the index just before a mint/swap/burn.
///
#[event]
pub struct IncreaseObservationCardinalityNext {
    /// The previous value of the next observation cardinality
    pub observation_cardinality_next_old: u16,

    /// The updated value of the next observation cardinality
    pub observation_cardinality_next_new: u16,
}

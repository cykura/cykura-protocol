use anchor_lang::prelude::*;

/// The pool state
///
/// PDA of `[token_0, token_1, fee]`
///
#[account(zero_copy)]
#[derive(Default)]
pub struct PoolState {
    /// Bump to identify PDA
    pub bump: u8,

    /// Token pair of the pool, where token_0 address < token_1 address
    pub token_0: Pubkey,
    pub token_1: Pubkey,

    /// Fee amount for swaps, denominated in hundredths of a bip (i.e. 1e-6)
    pub fee: u32,

    /// The minimum number of ticks between initialized ticks
    pub tick_spacing: u16,

    /// The currently in range liquidity available to the pool.
    /// This value has no relationship to the total liquidity across all ticks.
    pub liquidity: u64,

    /// The current price of the pool as a sqrt(token_1/token_0) Q64.96 value
    pub sqrt_price_x32: u64,

    /// The current tick of the pool, i.e. according to the last tick transition that was run.
    /// This value may not always be equal to SqrtTickMath.getTickAtSqrtRatio(sqrtPriceX96) if the
    /// price is on a tick boundary.
    /// Not necessarily a multiple of tick_spacing.
    pub tick: i32,

    /// The index of the last oracle observation that was written
    pub observation_index: u16,

    /// The current maximum number of observations stored in the pool
    pub observation_cardinality: u16,

    /// The next maximum number of observations, to be updated when an observation for a
    /// mint/swap/burn is recorded
    pub observation_cardinality_next: u16,

    /// The fee growth as a Q32.32 number, i.e. fees of token_0 and token_1 collected per
    /// unit of liquidity for the entire life of the pool.
    /// These values can overflow u64
    pub fee_growth_global_0_x32: u64,
    pub fee_growth_global_1_x32: u64,

    /// The current protocol fee as a percentage of the swap fee taken on withdrawal
    /// represented as an integer denominator (1/x)%
    /// Encoded as two 4 bit values, where the protocol fee of token_1 is shifted 4 bits and the
    /// protocol fee of token_0 is the lower 4 bits. Used as the denominator of a fraction of
    /// the swap fee, e.g. 4 means 1/4th of the swap fee.
    pub fee_protocol: u8,

    /// The amounts of token_0 and token_1 that are owed to the protocol.
    /// Protocol fees will never exceed u64::MAX in either token
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,

    /// Whether the pool is currently locked to reentrancy
    pub unlocked: bool,
}

impl PoolState {

}

/// Emitted when a pool is created and initialized with a starting price
///
#[event]
pub struct PoolCreatedAndInitialized {
    /// The first token of the pool by address sort order
    #[index]
    pub token_0: Pubkey,

    /// The second token of the pool by address sort order
    #[index]
    pub token_1: Pubkey,

    /// The fee collected upon every swap in the pool, denominated in hundredths of a bip
    #[index]
    pub fee: u32,

    /// The minimum number of ticks between initialized ticks
    pub tick_spacing: u16,

    /// The address of the created pool
    pub pool_state: Pubkey,

    /// The initial sqrt price of the pool, as a Q32.32
    pub sqrt_price_x32: u64,

    /// The initial tick of the pool, i.e. log base 1.0001 of the starting price of the pool
    pub tick: i32,
}

/// Emitted when the protocol fee is changed for a pool
#[event]
pub struct SetFeeProtocolEvent {
    /// The pool whose protocol fee is changed
    #[index]
    pub pool_state: Pubkey,

    /// The previous value of the token_0 protocol fee
    pub fee_protocol_0_old: u8,

    /// The previous value of the token_1 protocol fee
    pub fee_protocol_1_old: u8,

    /// The updated value of the token_0 protocol fee
    pub fee_protocol_0: u8,

    /// The updated value of the token_1 protocol fee
    pub fee_protocol_1: u8,
}

/// Emitted when the collected protocol fees are withdrawn by the factory owner
#[event]
pub struct CollectProtocolEvent {
    /// The pool whose protocol fee is collected
    #[index]
    pub pool_state: Pubkey,

    /// The address that collects the protocol fees
    #[index]
    pub sender: Pubkey,

    /// The address that receives the collected token_0 protocol fees
    pub recipient_wallet_0: Pubkey,

    /// The address that receives the collected token_1 protocol fees
    pub recipient_wallet_1: Pubkey,

    /// The amount of token_0 protocol fees that is withdrawn
    pub amount_0: u64,

    /// The amount of token_0 protocol fees that is withdrawn
    pub amount_1: u64,
}

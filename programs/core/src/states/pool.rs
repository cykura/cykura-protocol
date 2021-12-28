use anchor_lang::prelude::*;

/// Seed to derive account address and signature
pub const POOL_SEED: &str = "p";

/// The pool state
///
/// PDA of `[POOL_SEED, token_0, token_1, fee]`
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

/// Emitted by when a swap is performed for a pool
#[event]
pub struct SwapEvent {
    /// The pool for which token_0 and token_1 were swapped
    #[index]
    pub pool_state: Pubkey,

    /// The address that initiated the swap call, and that received the callback
    #[index]
    pub sender: Pubkey,

    /// The payer token account in zero for one swaps, or the recipient token account
    /// in one for zero swaps
    #[index]
    pub token_account_0: Pubkey,

    /// The payer token account in one for zero swaps, or the recipient token account
    /// in zero for one swaps
    #[index]
    pub token_account_1: Pubkey,

    /// The delta of the token_0 balance of the pool
    pub amount_0: i64,

    /// The delta of the token_1 balance of the pool
    pub amount_1: i64,

    /// The sqrt(price) of the pool after the swap, as a Q32.32
    pub sqrt_price_x32: u64,

    /// The liquidity of the pool after the swap
    pub liquidity: u64,

    /// The log base 1.0001 of price of the pool after the swap
    pub tick: i32
}
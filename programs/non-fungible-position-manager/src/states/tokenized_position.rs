use anchor_lang::prelude::*;

/// Position wrapped as an SPL non-fungible token
///
/// PDA of `[POSITION_SEED, mint_address]`
///
#[account(zero_copy)]
#[derive(Default)]
pub struct TokenizedPositionState {
    /// Bump to identify PDA
    pub bump: u8,

    /// Mint address of the tokenized position
    pub mint: Pubkey,

    /// The ID of the pool with which this token is connected
    pub pool_id: Pubkey,

    /// The lower bound tick of the position
    pub tick_lower: i32,

    /// The upper bound tick of the position
    pub tick_upper: i32,

    /// The amount of liquidity owned by this position
    pub liquidity: u64,

    /// The token_0 fee growth of the aggregate position as of the last action on the individual position
    pub fee_growth_inside_0_last_x32: u64,

    /// The token_0 fee growth of the aggregate position as of the last action on the individual position
    pub fee_growth_inside_1_last_x32: u64,

    /// How many uncollected token_0 are owed to the position, as of the last computation
    pub tokens_owed_0: u64,

    /// How many uncollected token_0 are owed to the position, as of the last computation
    pub tokens_owed_1: u64,
}

/// Emitted when liquidity is increased for a position NFT.
/// Also emitted when a token is minted
#[event]
pub struct IncreaseLiquidityEvent {
    /// The ID of the token for which liquidity was increased
    #[index]
    pub token_id: Pubkey,

    /// The amount by which liquidity for the NFT position was increased
    pub liquidity: u64,

    /// The amount of token_0 that was paid for the increase in liquidity
    pub amount_0: u64,

    /// The amount of token_1 that was paid for the increase in liquidity
    pub amount_1: u64,
}

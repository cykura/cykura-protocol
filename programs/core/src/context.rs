use crate::error::ErrorCode;
use crate::program::CyclosCore;
use crate::states::factory::FactoryState;
use crate::states::fee::{FeeState, FEE_SEED};
use crate::states::oracle::{ObservationState, OBSERVATION_SEED};
use crate::states::pool::{PoolState, POOL_SEED};
use crate::states::position::{PositionState, POSITION_SEED};
use crate::states::tick::{TickState, TICK_SEED};
use crate::states::tick_bitmap::{TickBitmapState, BITMAP_SEED};
use crate::states::tokenized_position::TokenizedPositionState;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::mem::size_of;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Initialize<'info> {
    /// Address to be set as protocol owner. It pays to create factory state account.
    pub owner: Signer<'info>,

    /// Initialize factory state account to store protocol owner address
    #[account(
        init,
        seeds = [],
        bump = bump,
        payer = owner,
    )]
    pub factory_state: Loader<'info, FactoryState>,

    /// To create a new program account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee_state_bump: u8, fee: u32, tick_spacing: u16)]
pub struct EnableFeeAmount<'info> {
    /// Valid protocol owner
    #[account(address = factory_state.load()?.owner)]
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Loader<'info, FactoryState>,

    /// Initialize an account to store new fee tier and tick spacing
    /// Fees are paid by owner
    #[account(
        init,
        seeds = [FEE_SEED.as_bytes(), &fee.to_be_bytes()],
        bump = fee_state_bump,
        payer = owner,
        space = 8 + size_of::<FeeState>()
    )]
    pub fee_state: Loader<'info, FeeState>,

    /// To create a new program account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOwner<'info> {
    /// Current protocol owner
    #[account(address = factory_state.load()?.owner)]
    pub owner: Signer<'info>,

    /// Address to be designated as new protocol owner
    pub new_owner: AccountInfo<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Loader<'info, FactoryState>,
}

#[derive(Accounts)]
#[instruction(pool_state_bump: u8, observation_state_bump: u8)]
pub struct CreateAndInitPool<'info> {
    /// Address paying to create the pool. Can be anyone
    #[account(mut)]
    pub pool_creator: Signer<'info>,

    /// Desired token pair for the pool
    /// token_0 mint address should be smaller than token_1 address
    #[account(
        constraint = token_0.key() < token_1.key()
    )]
    pub token_0: Box<Account<'info, Mint>>,
    pub token_1: Box<Account<'info, Mint>>,

    /// Stores the desired fee for the pool
    pub fee_state: Loader<'info, FeeState>,

    /// Initialize an account to store the pool state
    #[account(
        init,
        seeds = [
            POOL_SEED.as_bytes(),
            token_0.key().as_ref(),
            token_1.key().as_ref(),
            &fee_state.load()?.fee.to_be_bytes()
        ],
        bump = pool_state_bump,
        payer = pool_creator,
    )]
    pub pool_state: Loader<'info, PoolState>,

    /// Initialize an account to store oracle observations
    #[account(
        init,
        seeds = [
            &OBSERVATION_SEED.as_bytes(),
            token_0.key().as_ref(),
            token_1.key().as_ref(),
            &fee_state.load()?.fee.to_be_bytes(),
            &0_u16.to_be_bytes(),
        ],
        bump = observation_state_bump,
        payer = pool_creator,
    )]
    pub initial_observation_state: Loader<'info, ObservationState>,

    /// The address that holds pool tokens for token_0
    #[account(
        init_if_needed,
        associated_token::mint = token_0,
        associated_token::authority = pool_state,
        payer = pool_creator,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        init_if_needed,
        associated_token::mint = token_1,
        associated_token::authority = pool_state,
        payer = pool_creator,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// To create a new program account
    pub system_program: Program<'info, System>,

    /// Sysvar for program account and ATA creation
    pub rent: Sysvar<'info, Rent>,

    /// To create new token accounts for the pool
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct IncreaseObservationCardinalityNext<'info> {
    /// Pays to increase storage slots for oracle observations
    pub payer: Signer<'info>,

    /// Increase observation slots for this pool
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,

    /// To create new program accounts
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetFeeProtocol<'info> {
    /// Valid protocol owner
    #[account(address = factory_state.load()?.owner)]
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Loader<'info, FactoryState>,
}

#[derive(Accounts)]
pub struct CollectProtocol<'info> {
    /// Valid protocol owner
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Loader<'info, FactoryState>,

    /// Pool state stores accumulated protocol fee amount
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,

    /// The address that holds pool tokens for token_0
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_0.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_1.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The address that receives the collected token_0 protocol fees
    #[account(mut)]
    pub recipient_wallet_0: Box<Account<'info, TokenAccount>>,

    /// The address that receives the collected token_1 protocol fees
    #[account(mut)]
    pub recipient_wallet_1: Box<Account<'info, TokenAccount>>,

    /// The SPL program to perform token transfers
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(tick_account_bump: u8, tick: i32)]
pub struct InitTickAccount<'info> {
    /// Pays to create tick account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Create a tick account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The tick account to be initialized
    #[account(
        init,
        seeds = [
            TICK_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &tick.to_be_bytes()
        ],
        bump = tick_account_bump,
        payer = signer
    )]
    pub tick_state: Loader<'info, TickState>,

    /// Program to initialize the tick account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseTickAccount<'info> {
    /// The tick account to be initialized
    #[account(
        mut,
        close = recipient,
        constraint = tick_state.load()?.is_clear()
    )]
    pub tick_state: Loader<'info, TickState>,

    /// Destination for reclaimed lamports
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8, word_pos: i16)]
pub struct InitBitmapAccount<'info> {
    /// Pays to create bitmap account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Create a new bitmap account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The bitmap account to be initialized
    #[account(
        init,
        seeds = [
            BITMAP_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &word_pos.to_be_bytes()
        ],
        bump = bump,
        payer = signer
    )]
    pub bitmap_state: Loader<'info, TickBitmapState>,

    /// Program to initialize the tick account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitPositionAccount<'info> {
    /// Pays to create position account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The address of the position owner
    pub recipient: UncheckedAccount<'info>,

    /// Create a position account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The lower tick boundary of the position
    pub tick_lower_state: Loader<'info, TickState>,

    /// The upper tick boundary of the position
    #[account(
        constraint = tick_lower_state.load()?.tick < tick_upper_state.load()?.tick @ErrorCode::TLU
    )]
    pub tick_upper_state: Loader<'info, TickState>,

    /// The position account to be initialized
    #[account(
        init,
        seeds = [
            POSITION_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            recipient.key().as_ref(),
            &tick_lower_state.load()?.tick.to_be_bytes(),
            &tick_upper_state.load()?.tick.to_be_bytes(),
        ],
        bump = bump,
        payer = signer
    )]
    pub position_state: Loader<'info, PositionState>,

    /// Program to initialize the position account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintContext<'info> {
    /// Pays to mint liquidity
    #[account(mut)]
    pub minter: Signer<'info>,

    /// The token account spending token_0 to mint the position
    #[account(mut)]
    pub token_account_0: UncheckedAccount<'info>,

    /// The token account spending token_1 to mint the position
    #[account(mut)]
    pub token_account_1: UncheckedAccount<'info>,

    /// The address that holds pool tokens for token_0
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// Liquidity is minted on behalf of recipient
    pub recipient: UncheckedAccount<'info>,

    /// Mint liquidity for this pool
    #[account(mut)]
    // pub pool_state: UncheckedAccount<'info>,
    pub pool_state: Loader<'info, PoolState>,

    /// The lower tick boundary of the position
    #[account(mut)]
    pub tick_lower_state: Loader<'info, TickState>,

    /// The upper tick boundary of the position
    #[account(mut)]
    pub tick_upper_state: Loader<'info, TickState>,

    /// The bitmap storing initialization state of the lower tick
    #[account(mut)]
    pub bitmap_lower_state: UncheckedAccount<'info>,

    /// The bitmap storing initialization state of the upper tick
    #[account(mut)]
    pub bitmap_upper_state: UncheckedAccount<'info>,

    /// The position into which liquidity is minted
    #[account(mut)]
    pub position_state: UncheckedAccount<'info>,

    /// The program account for the most recent oracle observation, at index = pool.observation_index
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The SPL program to perform token transfers
    pub token_program: Program<'info, Token>,

    /// Program which receives mint_callback
    pub callback_handler: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintCallback<'info> {
    /// Pays to mint liquidity
    pub minter: Signer<'info>,

    /// The token account spending token_0 to mint the position
    pub token_account_0: UncheckedAccount<'info>,

    /// The token account spending token_1 to mint the position
    pub token_account_1: UncheckedAccount<'info>,

    /// The address that holds pool tokens for token_0
    pub vault_0: UncheckedAccount<'info>,

    /// The address that holds pool tokens for token_1
    pub vault_1: UncheckedAccount<'info>,

    /// The SPL program to perform token transfers
    pub token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SwapCallback<'info> {
    /// Pays for the swap
    pub signer: Signer<'info>,

    /// The user token account for input token
    #[account(mut)]
    pub input_token_account: UncheckedAccount<'info>,

    /// The user token account for output token
    #[account(mut)]
    pub output_token_account: UncheckedAccount<'info>,

    /// The vault token account for input token
    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,

    /// The vault token account for output token
    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,

    /// The SPL program to perform token transfers
    pub token_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct BurnContext<'info> {
    /// The position owner
    pub owner: Signer<'info>,

    /// Burn liquidity for this pool
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The lower tick boundary of the position
    pub tick_lower_state: UncheckedAccount<'info>,

    /// The upper tick boundary of the position
    pub tick_upper_state: UncheckedAccount<'info>,

    /// The bitmap storing initialization state of the lower tick
    pub bitmap_lower_state: UncheckedAccount<'info>,

    /// The bitmap storing initialization state of the upper tick
    pub bitmap_upper_state: UncheckedAccount<'info>,

    /// Burn liquidity from this position
    #[account(mut)]
    pub position_state: Loader<'info, PositionState>,

    /// The program account for the most recent oracle observation
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    pub next_observation_state: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CollectContext<'info> {
    /// The position owner
    pub owner: Signer<'info>,

    /// The program account for the liquidity pool from which fees are collected
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The lower tick of the position for which to collect fees
    pub tick_lower_state: UncheckedAccount<'info>,

    /// The upper tick of the position for which to collect fees
    pub tick_upper_state: UncheckedAccount<'info>,

    /// The position program account to collect fees from
    #[account(mut)]
    pub position_state: UncheckedAccount<'info>,

    /// The account holding pool tokens for token_0
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The account holding pool tokens for token_1
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The destination token account for the collected amount_0
    #[account(mut)]
    pub recipient_wallet_0: UncheckedAccount<'info>,

    /// The destination token account for the collected amount_1
    #[account(mut)]
    pub recipient_wallet_1: UncheckedAccount<'info>,

    /// SPL program to transfer out tokens
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SwapContext<'info> {
    /// The user performing the swap
    pub signer: Signer<'info>,

    /// The user token account for input token
    #[account(mut)]
    pub input_token_account: UncheckedAccount<'info>,

    /// The user token account for output token
    #[account(mut)]
    pub output_token_account: UncheckedAccount<'info>,

    /// The vault token account for input token
    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,

    /// The vault token account for output token
    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,

    /// SPL program for token transfers
    pub token_program: Program<'info, Token>,

    /// The factory state to read protocol fees
    pub factory_state: UncheckedAccount<'info>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The program account for the most recent oracle observation
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// Program which receives swap_callback
    pub callback_handler: UncheckedAccount<'info>,
}

// Non fungible position manager

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct MintTokenizedPosition<'info> {
    /// Pays to mint the position
    #[account(mut)]
    pub minter: Signer<'info>,

    /// Receives the position NFT
    pub recipient: UncheckedAccount<'info>,

    /// The program account acting as the core liquidity custodian for token holder, and as
    /// mint authority of the position NFT
    pub factory_state: Loader<'info, FactoryState>,

    /// Unique token mint address
    #[account(
        init,
        mint::decimals = 0,
        mint::authority = factory_state,
        payer = minter
    )]
    pub nft_mint: Box<Account<'info, Mint>>,

    /// Token account where position NFT will be minted
    #[account(
        init,
        associated_token::mint = nft_mint,
        associated_token::authority = recipient,
        payer = minter
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,

    /// Mint liquidity for this pool
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// Core program account to store position data
    #[account(mut)]
    pub core_position_state: UncheckedAccount<'info>,

    /// Account to store data for the position's lower tick
    #[account(mut)]
    pub tick_lower_state: UncheckedAccount<'info>,

    /// Account to store data for the position's upper tick
    #[account(mut)]
    pub tick_upper_state: UncheckedAccount<'info>,

    /// Account to mark the lower tick as initialized
    #[account(mut)]
    pub bitmap_lower_state: UncheckedAccount<'info>, // remove

    /// Account to mark the upper tick as initialized
    #[account(mut)]
    pub bitmap_upper_state: UncheckedAccount<'info>, // remove

    /// Metadata for the tokenized position
    #[account(
        init,
        seeds = [POSITION_SEED.as_bytes(), nft_mint.key().as_ref()],
        bump = bump,
        payer = minter
    )]
    pub tokenized_position_state: Loader<'info, TokenizedPositionState>,

    /// The token account spending token_0 to mint the position
    #[account(mut)]
    pub token_account_0: UncheckedAccount<'info>,

    /// The token account spending token_1 to mint the position
    #[account(mut)]
    pub token_account_1: UncheckedAccount<'info>,

    /// The token account owned by core to hold pool tokens for token_0
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The token account owned by core to hold pool tokens for token_1
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The latest observation state
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// Sysvar for token mint and ATA creation
    pub rent: Sysvar<'info, Rent>,

    /// The core program where liquidity is minted
    pub core_program: Program<'info, CyclosCore>,

    /// Program to create the position manager state account
    pub system_program: Program<'info, System>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,

    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct AddMetaplexMetadata<'info> {
    /// Pays to generate the metadata
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Authority of the NFT mint
    pub factory_state: Loader<'info, FactoryState>,

    /// Mint address for the tokenized position
    #[account(mut)]
    pub nft_mint: Box<Account<'info, Mint>>,

    /// Position state of the tokenized position
    #[account(
        seeds = [POSITION_SEED.as_bytes(), nft_mint.key().as_ref()],
        bump = tokenized_position_state.load()?.bump
    )]
    pub tokenized_position_state: Loader<'info, TokenizedPositionState>,

    /// To store metaplex metadata
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    /// Sysvar for metadata account creation
    pub rent: Sysvar<'info, Rent>,

    /// Program to create NFT metadata
    #[account(address = metaplex_token_metadata::ID)]
    pub metadata_program: UncheckedAccount<'info>,

    /// Program to update mint authority
    pub token_program: Program<'info, Token>,

    /// Program to allocate lamports to the metadata account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct IncreaseLiquidity<'info> {
    /// Pays to mint the position
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Authority PDA for the NFT mint
    pub factory_state: Loader<'info, FactoryState>,

    /// Increase liquidity for this position
    #[account(mut)]
    pub tokenized_position_state: Loader<'info, TokenizedPositionState>,

    /// Mint liquidity for this pool
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// Core program account to store position data
    #[account(mut)]
    pub core_position_state: UncheckedAccount<'info>,

    /// Account to store data for the position's lower tick
    #[account(mut)]
    pub tick_lower_state: UncheckedAccount<'info>,

    /// Account to store data for the position's upper tick
    #[account(mut)]
    pub tick_upper_state: UncheckedAccount<'info>,

    /// Stores init state for the lower tick
    #[account(mut)]
    pub bitmap_lower_state: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper_state: UncheckedAccount<'info>,

    /// The payer's token account for token_0
    #[account(mut)]
    pub token_account_0: UncheckedAccount<'info>,

    /// The payer's token account for token_1
    #[account(mut)]
    pub token_account_1: UncheckedAccount<'info>,

    /// The pool's token account for token_0
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The pool's token account for token_1
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The latest observation state
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The core program where liquidity is minted
    pub core_program: Program<'info, CyclosCore>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DecreaseLiquidity<'info> {
    /// The position owner or delegated authority
    #[account(mut)]
    pub owner_or_delegate: Signer<'info>,

    /// The token account for the tokenized position
    #[account(
        constraint = nft_account.mint == tokenized_position_state.load()?.mint
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,

    /// Decrease liquidity for this position
    #[account(mut)]
    pub tokenized_position_state: Loader<'info, TokenizedPositionState>,

    /// The program account acting as the core liquidity custodian for token holder
    pub factory_state: Loader<'info, FactoryState>,

    /// Burn liquidity for this pool
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// Core program account to store position data
    #[account(mut)]
    pub core_position_state: UncheckedAccount<'info>,

    /// Account to store data for the position's lower tick
    #[account(mut)]
    pub tick_lower_state: UncheckedAccount<'info>,

    /// Account to store data for the position's upper tick
    #[account(mut)]
    pub tick_upper_state: UncheckedAccount<'info>,

    /// Stores init state for the lower tick
    #[account(mut)]
    pub bitmap_lower_state: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper_state: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The core program where liquidity is burned
    pub core_program: Program<'info, CyclosCore>,
}

#[derive(Accounts)]
pub struct CollectFromTokenized<'info> {
    /// The position owner or delegated authority
    #[account(mut)]
    pub owner_or_delegate: Signer<'info>,

    /// The token account for the tokenized position
    #[account(
        constraint = nft_account.mint == tokenized_position_state.load()?.mint
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,

    /// The program account of the NFT for which tokens are being collected
    #[account(mut)]
    pub tokenized_position_state: Loader<'info, TokenizedPositionState>,

    /// The program account acting as the core liquidity custodian for token holder
    pub factory_state: Loader<'info, FactoryState>,

    /// The program account for the liquidity pool from which fees are collected
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The program account to access the core program position state
    #[account(mut)]
    pub core_position_state: UncheckedAccount<'info>,

    /// The program account for the position's lower tick
    #[account(mut)]
    pub tick_lower_state: UncheckedAccount<'info>,

    /// The program account for the position's upper tick
    #[account(mut)]
    pub tick_upper_state: UncheckedAccount<'info>,

    /// The bitmap program account for the init state of the lower tick
    #[account(mut)]
    pub bitmap_lower_state: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper_state: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The pool's token account for token_0
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The pool's token account for token_1
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The destination token account for the collected amount_0
    #[account(mut)]
    pub recipient_wallet_0: UncheckedAccount<'info>,

    /// The destination token account for the collected amount_1
    #[account(mut)]
    pub recipient_wallet_1: UncheckedAccount<'info>,

    /// The core program where liquidity is burned
    pub core_program: Program<'info, CyclosCore>,

    /// SPL program to transfer out tokens
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ExactInputSingle<'info> {
    /// The user performing the swap
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The factory state to read protocol fees
    pub factory_state: UncheckedAccount<'info>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The user token account for input token
    #[account(mut)]
    pub input_token_account: UncheckedAccount<'info>,

    /// The user token account for output token
    #[account(mut)]
    pub output_token_account: UncheckedAccount<'info>,

    /// The vault token account for input token
    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,

    /// The vault token account for output token
    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,

    /// The program account for the most recent oracle observation
    #[account(mut)]
    pub last_observation_state: UncheckedAccount<'info>,

    /// The account which follows the last observation, given by formula `(index_last + 1) % cardinality_next`
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The core program where swap is performed
    pub core_program: Program<'info, CyclosCore>,

    /// SPL program for token transfers
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ExactInput<'info> {
    /// The user performing the swap
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The factory state to read protocol fees
    pub factory_state: UncheckedAccount<'info>,

    #[account(mut)]
    pub input_token_account: UncheckedAccount<'info>,

    /// The core program where swap is performed
    pub core_program: Program<'info, CyclosCore>,

    /// SPL program for token transfers
    pub token_program: Program<'info, Token>,
}

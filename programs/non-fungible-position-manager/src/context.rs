use crate::states::position_manager::PositionManagerState;
use crate::{non_fungible_position_manager, states::tokenized_position::TokenizedPositionState};
use crate::error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use cyclos_core::states::pool::PoolState;
use cyclos_core::states::position::POSITION_SEED;

#[derive(Accounts)]
#[instruction(position_manager_state_bump: u8)]
pub struct Initialize<'info> {
    /// Pays to initialize the position manager state
    pub signer: Signer<'info>,

    /// Authority to mint position NFTs
    #[account(
        init,
        seeds = [],
        bump = position_manager_state_bump,
        payer = signer
    )]
    pub position_manager_state: Loader<'info, PositionManagerState>,

    /// Program to create the position manager state account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct MintPosition<'info> {
    /// Pays to mint the position
    #[account(mut)]
    pub minter: Signer<'info>,

    /// Receives the position NFT
    pub recipient: UncheckedAccount<'info>,

    /// The program account acting as the core liquidity custodian for token holder, and as
    /// mint authority of the position NFT
    pub position_manager_state: Loader<'info, PositionManagerState>,

    /// Unique token mint address
    #[account(
        init,
        mint::decimals = 0,
        mint::authority = position_manager_state,
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
    pub bitmap_lower: UncheckedAccount<'info>,

    /// Account to mark the upper tick as initialized
    #[account(mut)]
    pub bitmap_upper: UncheckedAccount<'info>,

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
    pub token_account_0: Box<Account<'info, TokenAccount>>,

    /// The token account spending token_1 to mint the position
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,

    /// The token account owned by core to hold pool tokens for token_0
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// The token account owned by core to hold pool tokens for token_1
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub latest_observation_state: UncheckedAccount<'info>,

    /// The next observation state
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// Sysvar for token mint and ATA creation
    pub rent: Sysvar<'info, Rent>,

    /// The core program where liquidity is minted
    pub core_program: Program<'info, cyclos_core::program::CyclosCore>,

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
    pub position_manager_state: Loader<'info, PositionManagerState>,

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
    pub position_manager_state: Loader<'info, PositionManagerState>,

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
    pub bitmap_lower: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper: UncheckedAccount<'info>,

    /// The payer's token account for token_0
    #[account(mut)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,

    /// The payer's token account for token_1
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,

    /// The pool's token account for token_0
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// The pool's token account for token_1
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub latest_observation_state: UncheckedAccount<'info>,

    /// The next observation state
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The core program where liquidity is minted
    pub core_program: Program<'info, cyclos_core::program::CyclosCore>,

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
    pub position_manager_state: Loader<'info, PositionManagerState>,

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
    pub bitmap_lower: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub latest_observation_state: UncheckedAccount<'info>,

    /// The next observation state
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The core program where liquidity is burned
    pub core_program: Program<'info, cyclos_core::program::CyclosCore>,
}

#[derive(Accounts)]
pub struct Collect<'info> {
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
    pub position_manager_state: Loader<'info, PositionManagerState>,

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
    pub bitmap_lower: UncheckedAccount<'info>,

    /// Stores init state for the upper tick
    #[account(mut)]
    pub bitmap_upper: UncheckedAccount<'info>,

    /// The latest observation state
    #[account(mut)]
    pub latest_observation_state: UncheckedAccount<'info>,

    /// The next observation state
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

    /// The account holding pool tokens for token_0
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// The account holding pool tokens for token_1
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// The destination token account for the collected amount_0
    #[account(mut)]
    pub recipient_wallet_0: UncheckedAccount<'info>,

    /// The destination token account for the collected amount_1
    #[account(mut)]
    pub recipient_wallet_1: UncheckedAccount<'info>,

    /// The core program where liquidity is burned
    pub core_program: Program<'info, cyclos_core::program::CyclosCore>,

    /// SPL program to transfer out tokens
    pub token_program: UncheckedAccount<'info>,
}
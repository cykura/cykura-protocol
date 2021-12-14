use crate::states::position_manager::PositionManagerState;
use crate::{
    non_fungible_position_manager, states::tokenized_position::TokenizedPositionState,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use cyclos_core::states::pool::PoolState;

pub const POSITION_SEED: &str = "p";

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

    /// Authority PDA for NFT mint
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

    /// Account to store metadata for NFT mint
    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

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

    /// Program to create NFT metadata
    #[account(address = metaplex_token_metadata::ID)]
    pub metadata_program: UncheckedAccount<'info>,

    /// Program to create the position manager state account
    pub system_program: Program<'info, System>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,

    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
}

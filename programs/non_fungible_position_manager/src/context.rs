use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{self, Mint, Token, TokenAccount}};
use cyclos_protocol_v2::states::pool::PoolState;
use crate::states::*;

#[derive(Accounts)]
#[instruction(position_manager_bump: u8)]
pub struct Init<'info> {
    pub signer: Signer<'info>,

    #[account(
        init,
        seeds = [],
        bump = position_manager_bump,
        payer = signer
    )]
    pub position_manager_state: Box<Account<'info, PositionManagerState>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintCallback<'info> {
    pub minter: Signer<'info>,

    // Should be a PDA of core contract
    // Core contract (factory in v3) must be passed via a constructor
    #[account(signer, owner = position_manager_state.core)]
    pub pool_state: AccountInfo<'info>,

    #[account(
        seeds = [],
        bump = position_manager_state.bump
    )]
    pub position_manager_state: Box<Account<'info, PositionManagerState>>,

    #[account(mut)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(
    non_fungible_position_bump: u8
)]
pub struct MintPosition<'info> {
    pub payer: Signer<'info>,
    pub recipient: AccountInfo<'info>,

    #[account(
        seeds = [],
        bump = position_manager_state.bump
    )]
    pub position_manager_state: Box<Account<'info, PositionManagerState>>,

    #[account(signer, owner = position_manager_state.core)]
    pub pool_state: Box<Account<'info, PoolState>>,

    // Validated and initialized inside core
    // TODO explore alternate way to init these, or need to pass seeds every time
    #[account(mut)]
    pub position_state: AccountInfo<'info>,
    #[account(mut)]
    pub tick_lower_state: AccountInfo<'info>,
    #[account(mut)]
    pub tick_upper_state: AccountInfo<'info>,
    #[account(mut)]
    pub tick_lower_bitmap: AccountInfo<'info>,
    #[account(mut)]
    pub tick_upper_bitmap: AccountInfo<'info>,

    // Randomly generated keypair
    #[account(
        init,
        mint::decimals = 0,
        mint::authority = position_manager_state,
        payer = payer
    )]
    pub nft_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        associated_token::mint = nft_mint,
        associated_token::authority = recipient,
        payer = payer
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        seeds = [nft_mint.key().as_ref()],
        bump = non_fungible_position_bump,
        payer = payer
    )]
    pub non_fungible_position_state: Box<Account<'info, NonFungiblePositionState>>,

    // Skip validation, performed during transfer
    #[account(mut)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = pool_state.token_0,
        associated_token::authority = pool_state,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = pool_state.token_1,
        associated_token::authority = pool_state,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

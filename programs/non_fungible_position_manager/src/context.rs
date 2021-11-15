use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use cyclos_protocol_v2::states::pool::PoolState;
use crate::cyclos_core;

#[derive(Accounts)]
pub struct MintCallback<'info> {
    pub minter: Signer<'info>,

    // Should be a PDA of core contract
    // Core contract (factory in v3) must be passed via a constructor
    #[account(signer, owner = cyclos_core::ID)]
    pub pool_state: AccountInfo<'info>,

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
pub struct MintAccount<'info> {
    pub minter: Signer<'info>,

    // Should be a PDA of core contract
    // Core contract (factory in v3) must be passed via a constructor
    #[account(signer, owner = cyclos_core::ID)]
    pub pool_state: Box<Account<'info, PoolState>>,

    pub recipient: AccountInfo<'info>,

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

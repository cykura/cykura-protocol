use crate::states::*;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount},
};
use cyclos_core::states::pool::PoolState;

#[derive(Accounts)]
pub struct SwapCallback {}

#[derive(Accounts)]
pub struct ExactInputSingle<'info> {
    /// The user performing the swap
    pub signer: Signer<'info>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The token account paying for the swap
    #[account(mut)]
    pub payer_token_account: Box<Account<'info, TokenAccount>>,

    /// The token account to receive the output of the swap
    #[account(mut)]
    pub recipient_token_account: Box<Account<'info, TokenAccount>>,

    /// The pool vault token account for token_0
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// The pool vault token account for token_1
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// The core program where swap is performed
    pub core_program: Program<'info, cyclos_core::program::CyclosCore>,

    /// SPL program for token transfers
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ExactInput {}

#[derive(Accounts)]
pub struct ExactOutputSingle {}

#[derive(Accounts)]
pub struct ExactOutput {}

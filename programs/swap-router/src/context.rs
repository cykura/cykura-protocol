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
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The program account of the pool in which the swap will be performed
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The payer token account in zero for one swaps, or the recipient token account
    /// in one for zero swaps
    #[account(mut)]
    pub token_account_0: UncheckedAccount<'info>,

    /// The payer token account in one for zero swaps, or the recipient token account
    /// in zero for one swaps
    #[account(mut)]
    pub token_account_1: UncheckedAccount<'info>,

    /// The pool vault token account for token_0
    #[account(mut)]
    pub vault_0: UncheckedAccount<'info>,

    /// The pool vault token account for token_1
    #[account(mut)]
    pub vault_1: UncheckedAccount<'info>,

    /// The program account for the most recent oracle observation
    #[account(mut)]
    pub latest_observation_state: UncheckedAccount<'info>,

    /// The observation program account one position after latest_observation_state
    #[account(mut)]
    pub next_observation_state: UncheckedAccount<'info>,

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

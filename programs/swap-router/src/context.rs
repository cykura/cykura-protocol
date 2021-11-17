use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{self, Mint, Token, TokenAccount}};
use cyclos_core::states::pool::PoolState;
use crate::states::*;

#[derive(Accounts)]
pub struct SwapCallback {}

#[derive(Accounts)]
pub struct ExactInputSingle {}

#[derive(Accounts)]
pub struct ExactInput {}

#[derive(Accounts)]
pub struct ExactOutputSingle {}

#[derive(Accounts)]
pub struct ExactOutput {}

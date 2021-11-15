pub mod cyclos_core;
pub mod libraries;
pub mod context;
use crate::context::*;
use cyclos_protocol_v2::libraries::tick_math;
use cyclos_protocol_v2::states::pool::PoolState;


use anchor_lang::prelude::*;
use anchor_spl::token::{self, SetAuthority, Token, TokenAccount, Transfer};
use libraries::liquidity_amounts::get_liquidity_for_amounts;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod non_fungible_position_manager {
    use super::*;

    pub fn mint_callback(
        ctx: Context<MintCallback>,
        amount_0_owed: u64,
        amount_1_owed: u64,
        data: [u8; 256],
    ) -> ProgramResult {
        // Transfer tokens from user to core program's vault_0
        if amount_0_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.token_account_0.to_account_info().clone(),
                        to: ctx.accounts.vault_0.to_account_info().clone(),
                        authority: ctx.accounts.minter.to_account_info().clone(),
                    },
                ),
                amount_0_owed,
            )?;
        }
        // Transfer tokens from user to core program's vault_1
        if amount_1_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.token_account_1.to_account_info().clone(),
                        to: ctx.accounts.vault_1.to_account_info().clone(),
                        authority: ctx.accounts.minter.to_account_info().clone(),
                    },
                ),
                amount_1_owed,
            )?;
        }
        Ok(())
    }

    pub fn mint(
        ctx: Context<MintAccount>,
        tick_lower: i32,
        tick_upper: i32,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64
        // TODO deadline
    ) -> ProgramResult {



        Ok(())
    }

}

// Add tokens to an initialized pool
// Tokens convert into liquidity depending on slippage
// @return liquidity Amount of liquidity added
// @return amount_0 Amount of token_0 consumed
// @return amount_1 Amount of token_1 consumed
pub fn add_liquidity<'info>(
    pool_state: &Account<'info, PoolState>,
    recipient: &AccountInfo<'info>,
    tick_lower: i32,
    tick_upper: i32,
    amount_0_desired: u64,
    amount_1_desired: u64,
    amount_0_min: u64,
    amount_1_min: u64
) -> (u32, u64, u64) {
    let sqrt_ratio_a = tick_math::get_sqrt_price_at_tick(tick_lower);
    let sqrt_ratio_b = tick_math::get_sqrt_price_at_tick(tick_upper);

    let liquidity = get_liquidity_for_amounts(pool_state.sqrt_price, sqrt_ratio_a, sqrt_ratio_b, amount_0_desired, amount_1_desired);

    // TODO CPI to mint
    // Possible to return values from CPI?

    // TODO slippage check
    (0,0,0)
}
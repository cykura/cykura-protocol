pub mod context;
pub mod error;
pub mod event;
pub mod states;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use context::*;
use cyclos_core::libraries::tick_math;
use error::ErrorCode;
use std::convert::TryFrom;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod swap_router {
    use super::*;

    /// Swaps as little as possible of one token for `amount_out` of another token,
    /// across a single pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - Token and pool accounts for swap
    /// * `zero_for_one` - Direction of swap. Swap token_0 for token_1 if true
    /// * `deadline` - Swap should if fail if past deadline
    /// * `amount_out` - Token amount to be swapped out
    /// * `amount_in_maximum` - For slippage. Panic if required input exceeds max limit.
    /// * `sqrt_price_limit` - Limit price √P for slippage
    ///
    pub fn exact_output_single(
        ctx: Context<ExactInputSingle>,
        zero_for_one: bool,
        deadline: u64,
        amount_out: u64,
        amount_in_maximum: u64,
        sqrt_price_limit_x32: u64,
    ) -> ProgramResult {
        todo!()
    }

    /// Swaps as little as possible of one token for `amount_out` of another
    /// along the specified path (reversed)
    ///
    /// # Arguments
    ///
    /// * `ctx` - Accounts for token transfer and swap route
    /// * `deadline` - Swap should if fail if past deadline
    /// * `amount_out` - Token amount to be swapped out
    /// * `amount_in_maximum` - For slippage. Panic if required input exceeds max limit.
    ///
    pub fn exact_output(
        ctx: Context<ExactInput>,
        deadline: u64,
        amount_out: u64,
        amount_out_maximum: u64,
    ) -> ProgramResult {
        todo!()
    }

    /// Swaps `amount_in` of one token for as much as possible of another token,
    /// across the path provided
    ///
    /// # Arguments
    ///
    /// * `ctx` - Accounts for token transfer and swap route
    /// * `deadline` - Swap should if fail if past deadline
    /// * `amount_in` - Token amount to be swapped in
    /// * `amount_out_minimum` - Panic if output amount is below minimum amount. For slippage.
    ///
    pub fn exact_input(
        ctx: Context<ExactInput>,
        deadline: u64,
        amount_in: u64,
        amount_out_minimum: u64,
    ) -> ProgramResult {
        todo!()
    }

    /// Swaps `amount_in` of one token for as much as possible of another token,
    /// across a single pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - Accounts required for the swap
    /// * `zero_for_one` -  The direction of the swap, true for token_0 to token_1, false for token_1 to token_0
    /// * `deadline` - The time by which the transaction must be included to effect the change
    /// * `amount_in` - Token amount to be swapped in
    /// * `amount_out_minimum` - The minimum amount to swap out, which serves as a slippage check
    /// * `sqrt_price_limit` - The Q32.32 sqrt price √P limit. If zero for one, the price cannot
    /// be less than this value after the swap.  If one for zero, the price cannot be greater than
    /// this value after the swap.
    ///
    #[access_control(check_deadline(deadline))]
    pub fn exact_input_single<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, ExactInputSingle<'info>>,
        deadline: i64,
        zero_for_one: bool,
        amount_in: u64,
        amount_out_minimum: u64,
        sqrt_price_limit_x32: u64,
    ) -> ProgramResult {
        let amount_out = exact_input_internal(
            ctx.accounts.core_program.to_account_info(),
            cyclos_core::cpi::accounts::SwapContext {
                signer: ctx.accounts.signer.to_account_info(),
                pool_state: ctx.accounts.pool_state.to_account_info(),
                token_account_0: ctx.accounts.token_account_0.to_account_info(),
                token_account_1: ctx.accounts.token_account_1.to_account_info(),
                vault_0: ctx.accounts.vault_0.to_account_info(),
                vault_1: ctx.accounts.vault_1.to_account_info(),
                latest_observation_state: ctx.accounts.latest_observation_state.to_account_info(),
                next_observation_state: ctx.accounts.next_observation_state.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                callback_handler: ctx.accounts.core_program.to_account_info()
            },
            ctx.remaining_accounts,
            zero_for_one,
            amount_in,
            sqrt_price_limit_x32,
        )?;
        require!(amount_out >= amount_out_minimum, ErrorCode::TooLittleReceived);
        Ok(())
    }
}

/// Performs a single exact input swap
pub fn exact_input_internal<'info>(
    core_program: AccountInfo<'info>,
    accounts: cyclos_core::cpi::accounts::SwapContext<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    zero_for_one: bool,
    amount_in: u64,
    sqrt_price_limit_x32: u64,
) -> Result<u64, ProgramError> {
    let mut vault = Account::<TokenAccount>::try_from(if zero_for_one {
        &accounts.vault_1
    } else {
        &accounts.vault_0
    })?;
    let balance_before = vault.amount;

    cyclos_core::cpi::swap(
        CpiContext::new(core_program, accounts)
            .with_remaining_accounts(remaining_accounts.to_vec()),
        zero_for_one,
        i64::try_from(amount_in).unwrap(),
        if sqrt_price_limit_x32 == 0 {
            if zero_for_one {
                tick_math::MIN_SQRT_RATIO + 1
            } else {
                tick_math::MAX_SQRT_RATIO - 1
            }
        } else {
            sqrt_price_limit_x32
        },
    )?;

    vault.reload()?;
    Ok(balance_before - vault.amount)
}

/// Common function to perform CPI for exact_output_single() and exact_output()
pub fn exact_output_internal() {
    todo!()
}

/// Checks whether the transaction time has not crossed the deadline
///
/// # Arguments
///
/// * `deadline` - The deadline specified by a user
///
pub fn check_deadline(deadline: i64) -> ProgramResult {
    require!(
        Clock::get()?.unix_timestamp <= deadline,
        ErrorCode::TransactionTooOld
    );
    Ok(())
}

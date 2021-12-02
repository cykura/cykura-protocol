pub mod context;
pub mod error;
pub mod event;
pub mod states;

use context::*;
use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod swap_router {
    use super::*;

    /// Callback for core.swap()
    /// Any contract that calls core.swap() must implement this interface
    /// Pay tokens owed for the swap to the pool
    /// Caller must be the core program
    ///
    /// # Flow
    ///
    /// 1. exact_input_internal() / exact_output_internal(): stateless routing
    /// 2. Core.UniswapV3Pool.swap(): transfer out resultant tokens to user
    /// 3. Periphery.SwapRouter.uniswapV3SwapCallback(): transfer tokens from user to pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - Token accounts to make payment
    /// * `amount_0_delta`, `amount_0_delta` - Δamount to be transferred to the pool
    /// * `data` - Arbitrary data field for third party integrators
    ///
    pub fn swap_callback(
        ctx: Context<SwapCallback>,
        amount_0_delta: i64,
        amount_1_delta: i64,
        data: [u128; 8]
    ) -> ProgramResult {

        todo!()
    }

    /// Swaps `amount_in` of one token for as much as possible of another token,
    /// across a single pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - Token and pool accounts for swap
    /// * `zero_for_one` - Direction of swap. Swap token_0 for token_1 if true
    /// * `deadline` - Swap should if fail if past deadline
    /// * `amount_in` - Token amount to be swapped in
    /// * `amount_out_minimum` - Panic if output amount is below minimum amount. For slippage.
    /// * `sqrt_price_limit` - Limit price √P for slippage
    ///
    pub fn exact_input_single(
        ctx: Context<ExactInputSingle>,
        zero_for_one: bool,
        deadline: u64,
        amount_in: u64,
        amount_out_minimum: u64,
        sqrt_price_limit_x32: u64
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
        sqrt_price_limit_x32: u64
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
}

/// Common function to perform CPI for exact_input_single() and exact_input()
pub fn exact_input_internal() {
    todo!()
}

/// Common function to perform CPI for exact_output_single() and exact_output()
pub fn exact_output_internal() {
    todo!()
}

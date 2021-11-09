pub mod context;
pub mod states;
pub mod libraries;
use crate::context::*;
use crate::states::factory::OwnerChangedEvent;
use crate::states::fee::FeeAmountEnabledEvent;
use anchor_lang::prelude::*;

declare_id!("37kn8WUzihQoAnhYxueA2BnqCA7VRnrVvYoHy1hQ6Veu");

#[program]
pub mod cyclos_protocol_v2 {
    use anchor_lang::solana_program::system_program;

    use super::*;

    // ---------------------------------------------------------------------
    // 1. Factory instructions

    pub fn initialize(ctx: Context<Initialize>, bump: u8) -> ProgramResult {
        ctx.accounts.factory_state.bump = bump;
        ctx.accounts.factory_state.owner = ctx.accounts.owner.key();

        emit!(OwnerChangedEvent {
            old_owner: system_program::ID,
            new_owner: ctx.accounts.owner.key(),
        });

        Ok(())
    }

    pub fn enable_fee_amount(
        ctx: Context<EnableFeeAmount>,
        fee: u32,
        tick_spacing: u16,
        fee_bump: u8,
    ) -> ProgramResult {
        if fee > 1_000_000 {
            // 100% fee
            return Err(ErrorCode::FeeLimit.into());
        }

        // TODO find why uni uses i24 and max 16384 for tickSpacing
        if tick_spacing > 16384 {
            return Err(ErrorCode::TickSpacingLimit.into());
        }

        emit!(FeeAmountEnabledEvent {
            fee, tick_spacing
        });

        ctx.accounts.fee_state.bump = fee_bump;
        ctx.accounts.fee_state.fee = fee;
        ctx.accounts.fee_state.tick_spacing = tick_spacing;

        Ok(())
    }

    pub fn set_owner(ctx: Context<Todo>) -> ProgramResult {
        todo!("Update owner and emit event. Read owner pubkey from ctx")
    }

    // ---------------------------------------------------------------------
    // 2. Pool instructions

    /// Create pool and initialize with desired price
    /// Create pool PDA for [token0, token1, fee] where tokenA > tokenB,
    /// then set sqrt_price
    /// Hardcode an initial protocol fee, not 0 like Uniswap
    ///
    /// Single function in place of Factory.createPool(), PoolDeployer.deploy()
    /// Pool.initialize() and pool.Constructor()
    pub fn create_pool(ctx: Context<Todo>, sqrt_price: f64) -> ProgramResult {
        todo!()
    }

    // ---------------------------------------------------------------------
    // 3. Position instructions

    /// Add liquidity for the given position
    /// Only callable by a smart contract which implements mintCallback()
    /// Periphery.LiquidityManagement.addLiquidity() -> Core.mint()
    ///     -> Periphery.LiquidityManagement.uniswapV3MintCallback()
    /// Due tokens must be paid in uniswapV3MintCallback()
    /// TODO study periphery and see what data field does
    pub fn mint(
        ctx: Context<Todo>,
        tick_lower: i32,
        tick_upper: i32,
        amount: u32,
    ) -> ProgramResult {
        // TODO convert tick_lower and tick_upper to i24
        todo!()
    }

    /// Collect tokens owed to a position
    /// Owed = fees + burned tokens
    /// 'Burned' tokens are tokens made inactive in a position, but are yet to be withdrawn
    /// Look at burn()
    /// Read position details (tick_upper, tick_lower) from the Position PDA
    pub fn collect(
        ctx: Context<Todo>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> ProgramResult {
        todo!()
    }

    /// Reduce liquidity in a position by given amount
    /// 'Burned' tokens are tokens made inactive in a position,
    /// but are not yet withdrawn
    pub fn burn(ctx: Context<Todo>, amount: u32) -> ProgramResult {
        todo!()
    }

    // ---------------------------------------------------------------------
    // 4. Swap instructions

    /// Perform swap
    ///
    /// Only callable by smart contract which implements uniswapV3SwapCallback()
    ///
    /// Flow
    /// 1. Periphery.SwapRouter.exactInputInternal()/exactOutputInternal(): stateless routing
    /// 2. Core.UniswapV3Pool.swap(): change state
    /// 3. Periphery.SwapRouter.uniswapV3SwapCallback(): transfer tokens from user to pool
    ///
    /// @param zero_for_one Swap token0 -> token1 if true, else token1 -> token0
    /// @param amount_specified Δtoken0 or Δtoken1 to be added/removed to pool.
    /// Exact input swap if positive, else exact output swap
    /// @param sqrt_price_limit Limit price √P for slippage
    pub fn swap(
        ctx: Context<Todo>,
        zero_for_one: bool,
        amount_specified: i64,
        sqrt_price_limit: f64,
    ) -> ProgramResult {
        todo!()
    }

    /// Component function for flash swaps
    ///
    /// Donate given liquidity to in-range positions then make callback
    /// Only callable by a smart contract which implements uniswapV3FlashCallback(),
    /// where profitability check can be performed
    ///
    /// Flash swaps is an advanced feature for developers, not directly available for UI based traders.
    /// Periphery does not provide an implementation, but a sample is provided
    /// Ref- https://github.com/Uniswap/v3-periphery/blob/main/contracts/examples/PairFlash.sol
    ///
    ///
    /// Flow
    /// 1. FlashDapp.initFlash()
    /// 2. Core.flash()
    /// 3. FlashDapp.uniswapV3FlashCallback()
    ///
    /// @param amount_0 Amount of token 0 to donate
    /// @param amount_1 Amount of token 1 to donate
    pub fn flash(ctx: Context<Todo>, amount_0: u64, amount_1: u64) -> ProgramResult {
        todo!()
    }

    // ---------------------------------------------------------------------
    // 5. Pool owner instructions

    /// Update protocol fees for a pool
    /// Protocol fee can be 0 or 1/N where 4 <= N <= 10 (fits in 4 bits)
    /// Both tokens in the pool can have different protocol fees
    /// Compress as a single u8, where fee_protocol_1 are leftmost bits and fee_protocol_0 are rightmost
    pub fn set_fee_protocol(
        ctx: Context<Todo>,
        fee_protocol_0: u8,
        fee_protocol_1: u8
    ) -> ProgramResult {
        todo!()
    }

    /// Collect protocol fees
    /// Amounts can be 0 to collect fees only in the other token
    pub fn collect_protocol(
        ctx: Context<Todo>,
        amount_0_requested: u64,
        amount_1_requested: u64
    ) -> ProgramResult {
        todo!()
    }

}

#[derive(Accounts)]
pub struct Todo {
}

// Error Codes
#[error]
pub enum ErrorCode {
    #[msg("Fees collected should be less than 1_000_000 (100%)")]
    FeeLimit,
    #[msg("Tick spacing should be less than 16384")]
    TickSpacingLimit,
}
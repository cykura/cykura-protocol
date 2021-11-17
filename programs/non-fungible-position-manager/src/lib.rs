pub mod libraries;
pub mod context;
pub mod error;
pub mod states;
use crate::context::*;
use cyclos_core::libraries::tick_math;
use cyclos_core::states::pool::PoolState;

use anchor_lang::prelude::*;
use error::ErrorCode;
use anchor_spl::token;
use libraries::liquidity_amounts::get_liquidity_for_amounts;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod non_fungible_position_manager {
    use super::*;

    /// Initialize the position manager
    /// Constructor to set bump and core program public key
    pub fn initialize(
        ctx: Context<Init>,
        position_manager_bump: u8,
        core: Pubkey
    ) -> ProgramResult {
        ctx.accounts.position_manager_state.bump = position_manager_bump;
        ctx.accounts.position_manager_state.core = core;
        Ok(())
    }

    /// Callback to pay tokens for creating or adding liquidity to a position
    ///
    /// create_position() / increase_liquidity() -> Core.mint() -> mint_callback()
    ///
    /// # Arguments
    ///
    /// * `amount_0_owed`, `amount_1_owed` - Amount of token_0 and token_0
    ///  to be transferred to pool
    /// * `data` - Arbitrary callback data. Not used by current manager.
    ///  Allow third party integrators to read additional data beyond the
    ///  available fields. use in tandem with remaining_accounts in context.
    ///
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

    /// Create a new position wrapped in an NFT
    /// Position manager acts as a proxy, owning all positions created on core.
    /// LPs in turn claim ownership through ownership of NFTs
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds pool and position accounts
    /// * `tick_lower`, `tick_upper` - Tick range for the position
    /// * `amount_0_desired`, `amount_1_desired` - Desired amounts of token_0 and token_1 to be added
    /// * `amount_0_min`, `amount_1_min`: - Mint fails if amounts added are below minimum levels
    /// * `deadline` - Mint fails if instruction is executed past the deadline
    ///
    pub fn mint(
        ctx: Context<MintPosition>,
        tick_lower: i32,
        tick_upper: i32,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: u64
    ) -> ProgramResult {
        require!(ctx.accounts.clock.slot <= deadline, ErrorCode::OldTransaction);

        // Call add_liquidity() to create position on core.mint()

        // Mint NFT- create token mint, ATA, and metaplex metadata

        // Write position data to PositionState PDA
        Ok(())
    }

    /// Increases liquidity in a position, with amount paid by `payer`
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds pool and position accounts
    /// * `amount_0_desired`, `amount_1_desired` - Desired amounts of token_0 and token_1 to be added
    /// * `amount_0_min`, `amount_1_min` - Mint fails if amounts added are below minimum levels
    /// * `deadline` - Mint fails if instruction is executed past the deadline
    ///
    pub fn increase_liquidity(
        ctx: Context<MintPosition>,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: u64
    ) -> ProgramResult {
        require!(ctx.accounts.clock.slot <= deadline, ErrorCode::OldTransaction);

        Ok(())
    }

    /// Decrease liquidity in a position and credit it as owed token amounts
    /// Liquidity provider must call collect() to claim owed tokens
    ///
    pub fn decrease_liquidity(
        ctx: Context<MintPosition>,
        liquidity: u32,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: u64
    ) -> ProgramResult {
        require!(ctx.accounts.clock.slot <= deadline, ErrorCode::OldTransaction);

        Ok(())
    }

    /// Collect owed fees upto the max specified amounts
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds position mint address and recipient address. Fees can be sent
    /// to third parties
    /// * `amount_0_max`, `amount_1_max` - Collect fees upto these amounts
    pub fn collect(
        ctx: Context<MintPosition>,
        amount_0_max: u64,
        amount_1_max: u64
    ) -> ProgramResult {

        // CPI core.burn() with amount 0 to trigger a poke, i.e. to update fee status
        // CPI core.collect() to collect fees from core and transfer to recipient

        todo!()
    }

    /// Burn a token to reclaim lamports
    /// Position must have zero liquidity and all tokens must be collected first
    pub fn burn(ctx: Context<MintPosition>) -> ProgramResult {
        // Accounts belonging to the program, SPL token and metaplex-metadata are closed
        // Transfer lamports to signer

        todo!()
    }
}

/// Internal function to add tokens to an initialized pool. Makes a CPI to
/// core.mint(). Returns a 3-tuple of liquidity, token_0 and token_1 consumed
///
/// Tokens convert into liquidity depending on slippage
///
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

    // CPI to core.mint()

    // TODO slippage check by reading balance from token accounts of minter

    (0,0,0)
}
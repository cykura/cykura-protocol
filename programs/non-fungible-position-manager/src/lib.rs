pub mod libraries;
pub mod context;
pub mod error;
pub mod states;
use states::position_manager;
use crate::context::*;
use cyclos_core::{cpi::accounts::MintContext, states::tick::TickState};
use cyclos_core::libraries::tick_math;
use anchor_lang::{prelude::*, solana_program::{instruction::Instruction, sysvar}};
use error::ErrorCode;
use libraries::liquidity_amounts;
use metaplex_token_metadata::instruction::{create_metadata_accounts, CreateMetadataAccountArgs};
use metaplex_token_metadata::{state::{Creator, Data}, instruction::MetadataInstruction};
use anchor_lang::solana_program::{self, system_instruction};
use anchor_spl::token;
use spl_token::instruction::AuthorityType;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");


pub const NFT_NAME: &str = "Uniswap Positions NFT-V1";
pub const NFT_SYMBOL: &str = "CYS-POS";
pub const BASE_URI: &str = "https://api.cyclos.io/mint=";

#[program]
pub mod non_fungible_position_manager {

    use cyclos_core::states::pool::PoolState;

    use super::*;

    /// Initializes the position manager by saving the core program address
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains core program address and initializes the position
    /// manager state account
    /// * `position_manager_state_bump` - Bump to validate the manager state address
    ///
    pub fn initialize(
        ctx: Context<Initialize>,
        position_manager_state_bump: u8
    ) -> ProgramResult {
        let position_manager_state = &mut ctx.accounts.position_manager_state.load_init()?;
        position_manager_state.bump = position_manager_state_bump;

        Ok(())
    }

    /// Creates a new position wrapped in a NFT
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds pool, tick, bitmap, position and token accounts
    /// * `amount_0_desired` - Desired amount of token_0 to be spent
    /// * `amount_1_desired` - Desired amount of token_1 to be spent
    /// * `amount_0_min` - The minimum amount of token_0 to spend, which serves as a slippage check
    /// * `amount_1_min` - The minimum amount of token_1 to spend, which serves as a slippage check
    /// * `deadline` - The time by which the transaction must be included to effect the change
    ///
    #[access_control(check_deadline(deadline))]
    pub fn mint(
        ctx: Context<MintPosition>,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: i64
    ) -> ProgramResult {
        let sqrt_price_x32 = Loader::<PoolState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.pool_state.to_account_info()
        )?.load()?.sqrt_price_x32;
        let tick_lower = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_lower_state.to_account_info()
        )?.load()?.tick;
        let tick_upper = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_upper_state.to_account_info()
        )?.load()?.tick;

        let sqrt_ratio_a_x32 = tick_math::get_sqrt_ratio_at_tick(tick_lower)?;
        let sqrt_ratio_b_x32 = tick_math::get_sqrt_ratio_at_tick(tick_upper)?;
        let liquidity = liquidity_amounts::get_liquidity_for_amounts(
            sqrt_price_x32,
            sqrt_ratio_a_x32,
            sqrt_ratio_b_x32,
            amount_0_desired,
            amount_1_desired
        );

        let seeds = [&[ctx.accounts.position_manager_state.load()?.bump] as &[u8]];
        let mint_accounts = MintContext {
            minter: ctx.accounts.minter.to_account_info(),
            recipient: ctx.accounts.position_manager_state.to_account_info(),
            pool_state: ctx.accounts.pool_state.to_account_info(),
            position_state: ctx.accounts.core_position_state.to_account_info(),
            tick_lower_state: ctx.accounts.tick_lower_state.to_account_info(),
            tick_upper_state: ctx.accounts.tick_upper_state.to_account_info(),
            bitmap_lower: ctx.accounts.bitmap_lower.to_account_info(),
            bitmap_upper: ctx.accounts.bitmap_upper.to_account_info(),
            token_account_0: ctx.accounts.token_account_0.to_account_info(),
            token_account_1: ctx.accounts.token_account_1.to_account_info(),
            vault_0: ctx.accounts.vault_0.to_account_info(),
            vault_1: ctx.accounts.vault_1.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            // system_program: ctx.accounts.system_program.to_account_info()
        };
        cyclos_core::cpi::mint(
            CpiContext::new_with_signer(
                ctx.accounts.core_program.to_account_info(),
                mint_accounts,
                &[&seeds[..]]
            ),
            liquidity
        )?;

        // Generate NFT metadata
        let create_metadata_ix = create_metadata_accounts(
            ctx.accounts.metadata_program.key(),
            ctx.accounts.metadata_account.key(),
            ctx.accounts.nft_mint.key(),
            ctx.accounts.position_manager_state.key(),
            ctx.accounts.minter.key(),
            ctx.accounts.position_manager_state.key(),
            NFT_NAME.to_string(),
            NFT_SYMBOL.to_string(),
            format!("{}{}", BASE_URI, ctx.accounts.nft_mint.key()),
            Some(vec![Creator {
                address: ctx.accounts.position_manager_state.key(),
                verified: true,
                share: 100,
            }]),
            0,
            true,
            false
        );
        solana_program::program::invoke_signed(
            &create_metadata_ix,
            &[
                ctx.accounts.metadata_account.to_account_info().clone(),
                ctx.accounts.nft_mint.to_account_info().clone(),
                ctx.accounts.minter.to_account_info().clone(), // payer
                ctx.accounts.position_manager_state.to_account_info().clone(), // mint and update authority
                ctx.accounts.system_program.to_account_info().clone(),
                ctx.accounts.rent.to_account_info().clone(),
            ],
            &[&seeds[..]]
        )?;

        // Mint the NFT
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            token::MintTo {
                mint: ctx.accounts.nft_mint.to_account_info().clone(),
                to: ctx.accounts.nft_account.to_account_info().clone(),
                authority: ctx.accounts.position_manager_state.to_account_info().clone(),
            },
            &[&seeds[..]]
        ), 1)?;

        // Disable minting
        token::set_authority(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(),
            token::SetAuthority {
                current_authority: ctx.accounts.position_manager_state.to_account_info().clone(),
                account_or_mint: ctx.accounts.nft_mint.to_account_info().clone(),
            },
            &[&seeds[..]]
        ), AuthorityType::MintTokens, None)?;

        Ok(())
    }


    // /// Callback to pay tokens for creating or adding liquidity to a position
    // ///
    // /// create_position() / increase_liquidity() -> Core.mint() -> mint_callback()
    // ///
    // /// # Arguments
    // ///
    // /// * `amount_0_owed`, `amount_1_owed` - Amount of token_0 and token_0
    // ///  to be transferred to pool
    // /// * `data` - Arbitrary callback data. Not used by current manager.
    // ///  Allow third party integrators to read additional data beyond the
    // ///  available fields. use in tandem with remaining_accounts in context.
    // ///
    // pub fn mint_callback(
    //     ctx: Context<MintCallback>,
    //     amount_0_owed: u64,
    //     amount_1_owed: u64,
    //     data: [u8; 256],
    // ) -> ProgramResult {
    //     // Transfer tokens from user to core program's vault_0
    //     if amount_0_owed > 0 {
    //         token::transfer(
    //             CpiContext::new(
    //                 ctx.accounts.token_program.to_account_info().clone(),
    //                 token::Transfer {
    //                     from: ctx.accounts.token_account_0.to_account_info().clone(),
    //                     to: ctx.accounts.vault_0.to_account_info().clone(),
    //                     authority: ctx.accounts.minter.to_account_info().clone(),
    //                 },
    //             ),
    //             amount_0_owed,
    //         )?;
    //     }
    //     // Transfer tokens from user to core program's vault_1
    //     if amount_1_owed > 0 {
    //         token::transfer(
    //             CpiContext::new(
    //                 ctx.accounts.token_program.to_account_info().clone(),
    //                 token::Transfer {
    //                     from: ctx.accounts.token_account_1.to_account_info().clone(),
    //                     to: ctx.accounts.vault_1.to_account_info().clone(),
    //                     authority: ctx.accounts.minter.to_account_info().clone(),
    //                 },
    //             ),
    //             amount_1_owed,
    //         )?;
    //     }
    //     Ok(())
    // }



    // /// Increases liquidity in a position, with amount paid by `payer`
    // ///
    // /// # Arguments
    // ///
    // /// * `ctx` - Holds pool and position accounts
    // /// * `amount_0_desired`, `amount_1_desired` - Desired amounts of token_0 and token_1 to be added
    // /// * `amount_0_min`, `amount_1_min` - Mint fails if amounts added are below minimum levels
    // /// * `deadline` - Mint fails if instruction is executed past the deadline
    // ///
    // pub fn increase_liquidity(
    //     ctx: Context<MintPosition>,
    //     amount_0_desired: u64,
    //     amount_1_desired: u64,
    //     amount_0_min: u64,
    //     amount_1_min: u64,
    //     deadline: u64
    // ) -> ProgramResult {
    //     require!(ctx.accounts.clock.slot <= deadline, ErrorCode::OldTransaction);

    //     Ok(())
    // }

    // /// Decrease liquidity in a position and credit it as owed token amounts
    // /// Liquidity provider must call collect() to claim owed tokens
    // ///
    // pub fn decrease_liquidity(
    //     ctx: Context<MintPosition>,
    //     liquidity: u32,
    //     amount_0_min: u64,
    //     amount_1_min: u64,
    //     deadline: u64
    // ) -> ProgramResult {
    //     require!(ctx.accounts.clock.slot <= deadline, ErrorCode::OldTransaction);

    //     Ok(())
    // }

    // /// Collect owed fees upto the max specified amounts
    // ///
    // /// # Arguments
    // ///
    // /// * `ctx` - Holds position mint address and recipient address. Fees can be sent
    // /// to third parties
    // /// * `amount_0_max`, `amount_1_max` - Collect fees upto these amounts
    // pub fn collect(
    //     ctx: Context<MintPosition>,
    //     amount_0_max: u64,
    //     amount_1_max: u64
    // ) -> ProgramResult {

    //     // CPI core.burn() with amount 0 to trigger a poke, i.e. to update fee status
    //     // CPI core.collect() to collect fees from core and transfer to recipient

    //     todo!()
    // }

    // /// Burn a token to reclaim lamports
    // /// Position must have zero liquidity and all tokens must be collected first
    // pub fn burn(ctx: Context<MintPosition>) -> ProgramResult {
    //     // Accounts belonging to the program, SPL token and metaplex-metadata are closed
    //     // Transfer lamports to signer

    //     todo!()
    // }
}

/// Checks whether the transaction time has not crossed the deadline
///
/// # Arguments
///
/// * `deadline` - The deadline specified by a user
///
pub fn check_deadline(deadline: i64) -> ProgramResult {
    require!(Clock::get()?.unix_timestamp <= deadline, ErrorCode::TransactionTooOld);
    Ok(())
}

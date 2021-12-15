pub mod context;
pub mod error;
pub mod libraries;
pub mod states;
use crate::context::*;
use anchor_lang::solana_program::{self, system_instruction};
use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, sysvar},
};
use anchor_spl::token::{self, Token, TokenAccount};
use cyclos_core::libraries::tick_math;
use cyclos_core::states::pool::PoolState;
use cyclos_core::{cpi::accounts::MintContext, states::tick::TickState};
use error::ErrorCode;
use libraries::liquidity_amounts;
use metaplex_token_metadata::instruction::{create_metadata_accounts, CreateMetadataAccountArgs};
use metaplex_token_metadata::{
    instruction::MetadataInstruction,
    state::{Creator, Data},
};
use spl_token::instruction::AuthorityType;
use states::position_manager::{self, PositionManagerState};
extern crate muldiv;
use muldiv::MulDiv;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod non_fungible_position_manager {

    use std::ops::Deref;

    use cyclos_core::{
        cpi::accounts::BurnContext,
        libraries::fixed_point_x32,
        states::{pool::PoolState, position::PositionState},
    };
    use states::tokenized_position::{
        DecreaseLiquidityEvent, IncreaseLiquidityEvent, TokenizedPositionState,
    };

    use super::*;

    /// Initializes the position manager by saving the core program address
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains core program address and initializes the position
    /// manager state account
    /// * `position_manager_state_bump` - Bump to validate the manager state address
    ///
    pub fn initialize(ctx: Context<Initialize>, position_manager_state_bump: u8) -> ProgramResult {
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
        bump: u8,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: i64,
    ) -> ProgramResult {
        let tick_lower = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_lower_state.to_account_info(),
        )?
        .load()?
        .tick;

        let tick_upper = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_upper_state.to_account_info(),
        )?
        .load()?
        .tick;

        let (liquidity, amount_0, amount_1) = add_liquidity(
            amount_0_desired,
            amount_1_desired,
            amount_0_min,
            amount_1_min,
            tick_lower,
            tick_upper,
            ctx.accounts.minter.to_account_info(),
            ctx.accounts.position_manager_state.to_account_info(),
            ctx.accounts.pool_state.to_account_info(),
            ctx.accounts.core_position_state.to_account_info(),
            ctx.accounts.tick_lower_state.to_account_info(),
            ctx.accounts.tick_upper_state.to_account_info(),
            ctx.accounts.bitmap_lower.to_account_info(),
            ctx.accounts.bitmap_upper.to_account_info(),
            &mut ctx.accounts.token_account_0,
            &mut ctx.accounts.token_account_1,
            ctx.accounts.vault_0.to_account_info(),
            ctx.accounts.vault_1.to_account_info(),
            ctx.accounts.latest_observation_state.to_account_info(),
            ctx.accounts.next_observation_state.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.core_program.to_account_info(),
        )?;

        // Mint the NFT
        let seeds = [&[ctx.accounts.position_manager_state.load()?.bump] as &[u8]];
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                token::MintTo {
                    mint: ctx.accounts.nft_mint.to_account_info().clone(),
                    to: ctx.accounts.nft_account.to_account_info().clone(),
                    authority: ctx
                        .accounts
                        .position_manager_state
                        .to_account_info()
                        .clone(),
                },
                &[&seeds[..]],
            ),
            1,
        )?;

        // Write tokenized position metadata
        let mut tokenized_position = ctx.accounts.tokenized_position_state.load_init()?;
        tokenized_position.bump = bump;
        tokenized_position.mint = ctx.accounts.nft_mint.key();
        tokenized_position.pool_id = ctx.accounts.pool_state.key();
        tokenized_position.tick_lower = tick_lower;
        tokenized_position.tick_upper = tick_upper;
        tokenized_position.liquidity = liquidity;
        tokenized_position.fee_growth_inside_0_last_x32 = Loader::<PositionState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.core_position_state.to_account_info(),
        )?
        .load()?
        .fee_growth_inside_0_last_x32;
        tokenized_position.fee_growth_inside_0_last_x32 = Loader::<PositionState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.core_position_state.to_account_info(),
        )?
        .load()?
        .fee_growth_inside_1_last_x32;

        emit!(IncreaseLiquidityEvent {
            token_id: ctx.accounts.nft_mint.key(),
            liquidity,
            amount_0,
            amount_1
        });

        Ok(())
    }

    /// Attach metaplex metadata to a tokenized position. Permissionless to call.
    /// Optional and cosmetic in nature.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds validated metadata account and tokenized position addresses
    ///
    pub fn add_metaplex_metadata(ctx: Context<AddMetaplexMetadata>) -> ProgramResult {
        let seeds = [&[ctx.accounts.position_manager_state.load()?.bump] as &[u8]];
        let create_metadata_ix = create_metadata_accounts(
            ctx.accounts.metadata_program.key(),
            ctx.accounts.metadata_account.key(),
            ctx.accounts.nft_mint.key(),
            ctx.accounts.position_manager_state.key(),
            ctx.accounts.payer.key(),
            ctx.accounts.position_manager_state.key(),
            String::from("Cyclos Positions NFT-V1"),
            String::from("CYS-POS"),
            format!("https://api.cyclos.io/mint={}", ctx.accounts.nft_mint.key()),
            Some(vec![Creator {
                address: ctx.accounts.position_manager_state.key(),
                verified: true,
                share: 100,
            }]),
            0,
            true,
            false,
        );
        solana_program::program::invoke_signed(
            &create_metadata_ix,
            &[
                ctx.accounts.metadata_account.to_account_info().clone(),
                ctx.accounts.nft_mint.to_account_info().clone(),
                ctx.accounts.payer.to_account_info().clone(),
                ctx.accounts
                    .position_manager_state
                    .to_account_info()
                    .clone(), // mint and update authority
                ctx.accounts.system_program.to_account_info().clone(),
                ctx.accounts.rent.to_account_info().clone(),
            ],
            &[&seeds[..]],
        )?;

        // Disable minting
        token::set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info().clone(),
                token::SetAuthority {
                    current_authority: ctx
                        .accounts
                        .position_manager_state
                        .to_account_info()
                        .clone(),
                    account_or_mint: ctx.accounts.nft_mint.to_account_info().clone(),
                },
                &[&seeds[..]],
            ),
            AuthorityType::MintTokens,
            None,
        )?;

        Ok(())
    }

    /// Increases liquidity in a position, with amount paid by `payer`
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds the pool, tick, bitmap, position and token accounts
    /// * `amount_0_desired` - Desired amount of token_0 to be spent
    /// * `amount_1_desired` - Desired amount of token_1 to be spent
    /// * `amount_0_min` - The minimum amount of token_0 to spend, which serves as a slippage check
    /// * `amount_1_min` - The minimum amount of token_1 to spend, which serves as a slippage check
    /// * `deadline` - The time by which the transaction must be included to effect the change
    ///
    #[access_control(check_deadline(deadline))]
    pub fn increase_liquidity(
        ctx: Context<IncreaseLiquidity>,
        amount_0_desired: u64,
        amount_1_desired: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: i64,
    ) -> ProgramResult {
        let tick_lower = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_lower_state.to_account_info(),
        )?
        .load()?
        .tick;

        let tick_upper = Loader::<TickState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.tick_upper_state.to_account_info(),
        )?
        .load()?
        .tick;

        let (liquidity, amount_0, amount_1) = add_liquidity(
            amount_0_desired,
            amount_1_desired,
            amount_0_min,
            amount_1_min,
            tick_lower,
            tick_upper,
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.position_manager_state.to_account_info(),
            ctx.accounts.pool_state.to_account_info(),
            ctx.accounts.core_position_state.to_account_info(),
            ctx.accounts.tick_lower_state.to_account_info(),
            ctx.accounts.tick_upper_state.to_account_info(),
            ctx.accounts.bitmap_lower.to_account_info(),
            ctx.accounts.bitmap_upper.to_account_info(),
            &mut ctx.accounts.token_account_0,
            &mut ctx.accounts.token_account_1,
            ctx.accounts.vault_0.to_account_info(),
            ctx.accounts.vault_1.to_account_info(),
            ctx.accounts.latest_observation_state.to_account_info(),
            ctx.accounts.next_observation_state.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.core_program.to_account_info(),
        )?;

        let core_position_loader = Loader::<PositionState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.core_position_state.to_account_info(),
        )?;
        let fee_growth_inside_0_last_x32 =
            core_position_loader.load()?.fee_growth_inside_0_last_x32;
        let fee_growth_inside_1_last_x32 =
            core_position_loader.load()?.fee_growth_inside_1_last_x32;

        // Update tokenized position metadata
        let mut position = ctx.accounts.tokenized_position_state.load_mut()?;
        position.tokens_owed_0 += (fee_growth_inside_0_last_x32
            - position.fee_growth_inside_0_last_x32)
            .mul_div_floor(position.liquidity, fixed_point_x32::Q32)
            .unwrap();

        position.tokens_owed_1 += (fee_growth_inside_1_last_x32
            - position.fee_growth_inside_1_last_x32)
            .mul_div_floor(position.liquidity, fixed_point_x32::Q32)
            .unwrap();

        position.fee_growth_inside_0_last_x32 = fee_growth_inside_0_last_x32;
        position.fee_growth_inside_0_last_x32 = fee_growth_inside_1_last_x32;
        position.liquidity += liquidity;

        emit!(IncreaseLiquidityEvent {
            token_id: position.mint,
            liquidity,
            amount_0,
            amount_1
        });

        Ok(())
    }

    /// Decreases the amount of liquidity in a position and accounts it to the position
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds the pool, tick, bitmap, position and token accounts
    /// * `liquidity` - The amount by which liquidity will be decreased
    /// * `amount_0_min` - The minimum amount of token_0 that should be accounted for the burned liquidity
    /// * `amount_1_min` - The minimum amount of token_1 that should be accounted for the burned liquidity
    /// * `deadline` - The time by which the transaction must be included to effect the change
    ///
    #[access_control(check_deadline(deadline))]
    #[access_control(is_authorized_for_token(&ctx.accounts.owner_or_delegate, &ctx.accounts.nft_account))]
    pub fn decrease_liquidity(
        ctx: Context<DecreaseLiquidity>,
        liquidity: u64,
        amount_0_min: u64,
        amount_1_min: u64,
        deadline: i64,
    ) -> ProgramResult {
        assert!(liquidity > 0);

        let core_position_loader = Loader::<PositionState>::try_from(
            &cyclos_core::id(),
            &ctx.accounts.core_position_state.to_account_info(),
        )?;
        let tokens_owed_0_before = core_position_loader.load()?.tokens_owed_0;
        let tokens_owed_1_before = core_position_loader.load()?.tokens_owed_1;

        cyclos_core::cpi::burn(
            CpiContext::new_with_signer(
                ctx.accounts.core_program.to_account_info(),
                BurnContext {
                    owner: ctx.accounts.position_manager_state.to_account_info(),
                    lamport_destination: ctx.accounts.owner_or_delegate.to_account_info(),
                    pool_state: ctx.accounts.pool_state.to_account_info(),
                    tick_lower_state: ctx.accounts.tick_lower_state.to_account_info(),
                    tick_upper_state: ctx.accounts.tick_upper_state.to_account_info(),
                    bitmap_lower: ctx.accounts.bitmap_lower.to_account_info(),
                    bitmap_upper: ctx.accounts.bitmap_upper.to_account_info(),
                    position_state: ctx.accounts.core_position_state.to_account_info(),
                    latest_observation_state: ctx
                        .accounts
                        .latest_observation_state
                        .to_account_info(),
                    next_observation_state: ctx.accounts.next_observation_state.to_account_info(),
                },
                &[&[&[ctx.accounts.position_manager_state.load()?.bump]]],
            ),
            liquidity,
        )?;

        let amount_0 = core_position_loader.load()?.tokens_owed_0 - tokens_owed_0_before;
        let amount_1 = core_position_loader.load()?.tokens_owed_1 - tokens_owed_1_before;
        msg!("amount 0 {}, amount 1 {}", amount_0, amount_1);
        require!(
            amount_0 >= amount_0_min && amount_1 >= amount_1_min,
            ErrorCode::PriceSlippageCheck
        );

        // Update the tokenized position to the current transaction
        let fee_growth_inside_0_last_x32 =
            core_position_loader.load()?.fee_growth_inside_0_last_x32;
        let fee_growth_inside_1_last_x32 =
            core_position_loader.load()?.fee_growth_inside_1_last_x32;

        let mut position = ctx.accounts.tokenized_position_state.load_mut()?;
        position.tokens_owed_0 += amount_0
            + (fee_growth_inside_0_last_x32 - position.fee_growth_inside_0_last_x32)
                .mul_div_floor(position.liquidity, fixed_point_x32::Q32)
                .unwrap();

        position.tokens_owed_1 += amount_1
            + (fee_growth_inside_1_last_x32 - position.fee_growth_inside_1_last_x32)
                .mul_div_floor(position.liquidity, fixed_point_x32::Q32)
                .unwrap();

        position.fee_growth_inside_0_last_x32 = fee_growth_inside_0_last_x32;
        position.fee_growth_inside_0_last_x32 = fee_growth_inside_1_last_x32;
        position.liquidity -= liquidity;

        emit!(DecreaseLiquidityEvent {
            token_id: position.mint,
            liquidity,
            amount_0,
            amount_1
        });

        Ok(())
    }

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

/// Add liquidity to an initialized pool
///
/// # Arguments
///
/// * `amount_0_desired` - Desired amount of token_0 to be spent
/// * `amount_1_desired` - Desired amount of token_1 to be spent
/// * `amount_0_min` - The minimum amount of token_0 to spend, which serves as a slippage check
/// * `amount_1_min` - The minimum amount of token_1 to spend, which serves as a slippage check
/// * `tick_lower` - The lower tick bound for the position
/// * `tick_upper` - The upper tick bound for the position
/// * `minter` - Pays to mint liquidity
/// * `recipient` - The recipient of the minted liquidity
/// * `pool_state` - Mint liquidity to this pool
/// * `core_position_state` - The core program position account where liquidity is minted
/// * `tick_lower_state` - The lower tick account
/// * `tick_upper_state` - The upper tick account
/// * `bitmap_lower` - Holds init state for the lower tick
/// * `bitmap_upper` - Holds init state for the upper tick
/// * `token_account_0` - The account spending amount_0
/// * `token_account_1` - The account spending amount_1
/// * `vault_0` - Token account for token_0 owned by the pool
/// * `vault_1` - Token account for token_1 owned by the pool
/// * `latest_observation_state` - The latest observation at observation_index
/// * `next_observation_state` - The account at observation_index + 1, wrapped by cardinality
/// * `token_program` - The SPL program to perform token transfers
/// * `core_program` - The core program where liquidity is minted
///
pub fn add_liquidity<'info>(
    amount_0_desired: u64,
    amount_1_desired: u64,
    amount_0_min: u64,
    amount_1_min: u64,
    tick_lower: i32,
    tick_upper: i32,
    minter: AccountInfo<'info>,
    recipient: AccountInfo<'info>,
    pool_state: AccountInfo<'info>,
    core_position_state: AccountInfo<'info>,
    tick_lower_state: AccountInfo<'info>,
    tick_upper_state: AccountInfo<'info>,
    bitmap_lower: AccountInfo<'info>,
    bitmap_upper: AccountInfo<'info>,
    token_account_0: &mut Box<Account<'info, TokenAccount>>,
    token_account_1: &mut Box<Account<'info, TokenAccount>>,
    vault_0: AccountInfo<'info>,
    vault_1: AccountInfo<'info>,
    latest_observation_state: AccountInfo<'info>,
    next_observation_state: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    core_program: AccountInfo<'info>,
) -> Result<(u64, u64, u64), ProgramError> {
    let sqrt_price_x32 =
        Loader::<PoolState>::try_from(&cyclos_core::id(), &pool_state.to_account_info())?
            .load()?
            .sqrt_price_x32;

    let sqrt_ratio_a_x32 = tick_math::get_sqrt_ratio_at_tick(tick_lower)?;
    let sqrt_ratio_b_x32 = tick_math::get_sqrt_ratio_at_tick(tick_upper)?;
    let liquidity = liquidity_amounts::get_liquidity_for_amounts(
        sqrt_price_x32,
        sqrt_ratio_a_x32,
        sqrt_ratio_b_x32,
        amount_0_desired,
        amount_1_desired,
    );

    let balance_0_before = token_account_0.amount;
    let balance_1_before = token_account_1.amount;

    let mint_accounts = MintContext {
        minter,
        recipient,
        pool_state,
        position_state: core_position_state,
        tick_lower_state,
        tick_upper_state,
        bitmap_lower,
        bitmap_upper,
        token_account_0: token_account_0.to_account_info(),
        token_account_1: token_account_1.to_account_info(),
        latest_observation_state,
        next_observation_state,
        vault_0,
        vault_1,
        token_program,
        callback_handler: core_program.clone(),
    };

    cyclos_core::cpi::mint(CpiContext::new(core_program, mint_accounts), liquidity)?;

    token_account_0.reload()?;
    token_account_1.reload()?;
    let amount_0 = balance_0_before - token_account_0.amount;
    let amount_1 = balance_1_before - token_account_1.amount;
    require!(
        amount_0 >= amount_0_min && amount_1 >= amount_1_min,
        ErrorCode::PriceSlippageCheck
    );

    Ok((liquidity, amount_0, amount_1))
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

pub fn is_authorized_for_token<'info>(
    signer: &Signer<'info>,
    token_account: &Box<Account<'info, TokenAccount>>,
) -> ProgramResult {
    require!(
        token_account.owner == signer.key() || (
            token_account.delegate.contains(&signer.key())
                && token_account.delegated_amount > 0
        ),
        ErrorCode::NotApproved
    );
    Ok(())
}

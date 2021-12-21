pub mod context;
pub mod error;
pub mod libraries;
pub mod states;
use crate::error::ErrorCode;
use crate::libraries::tick_math;
use crate::states::oracle;
use crate::states::oracle::ObservationState;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::solana_program::system_instruction::create_account;
use anchor_lang::AccountsClose;
use anchor_lang::{solana_program::instruction::Instruction, InstructionData};
use anchor_spl::token;
use context::*;
use libraries::liquidity_math;
use libraries::sqrt_price_math;
use muldiv::MulDiv;
use states::factory::*;
use states::fee::*;
use states::pool::*;
use states::position::*;
use states::tick::*;
use states::tick_bitmap::*;
use std::convert::TryFrom;
use std::mem::size_of;
use std::ops::Neg;
use std::ops::{Deref, DerefMut};

use crate::{
    libraries::{fixed_point_x32, swap_math},
    states::{oracle::OBSERVATION_SEED, tick_bitmap},
};

declare_id!("37kn8WUzihQoAnhYxueA2BnqCA7VRnrVvYoHy1hQ6Veu");

#[program]
pub mod cyclos_core {
    use super::*;

    // ---------------------------------------------------------------------
    // Factory instructions
    // The Factory facilitates creation of pools and control over the protocol fees

    /// Initialize the factory state and set the protocol owner
    ///
    /// # Arguments
    ///
    /// * `ctx`- Initializes the factory state account
    /// * `factory_state_bump` - Bump to validate factory state address
    ///
    pub fn init_factory(ctx: Context<Initialize>, factory_state_bump: u8) -> ProgramResult {
        let mut factory_state = ctx.accounts.factory_state.load_init()?;
        factory_state.bump = factory_state_bump;
        factory_state.owner = ctx.accounts.owner.key();

        emit!(OwnerChanged {
            old_owner: Pubkey::default(),
            new_owner: ctx.accounts.owner.key(),
        });

        Ok(())
    }

    /// Updates the owner of the factory
    /// Must be called by the current owner
    ///
    /// # Arguments
    ///
    /// * `ctx`- Checks whether protocol owner has signed
    ///
    pub fn set_owner(ctx: Context<SetOwner>) -> ProgramResult {
        let mut factory_state = ctx.accounts.factory_state.load_mut()?;
        factory_state.owner = ctx.accounts.new_owner.key();

        emit!(OwnerChanged {
            old_owner: ctx.accounts.owner.key(),
            new_owner: ctx.accounts.new_owner.key(),
        });

        Ok(())
    }

    /// Enables a fee amount with the given tick_spacing
    /// Fee amounts may never be removed once enabled
    ///
    /// # Arguments
    ///
    /// * `ctx`- Checks whether protocol owner has signed and initializes the fee account
    /// * `fee_state_bump` - Bump to validate fee state address
    /// * `fee` - The fee amount to enable, denominated in hundredths of a bip (i.e. 1e-6)
    /// * `tick_spacing` - The spacing between ticks to be enforced for all pools created
    /// with the given fee amount
    ///
    pub fn enable_fee_amount(
        ctx: Context<EnableFeeAmount>,
        fee_state_bump: u8,
        fee: u32,
        tick_spacing: u16,
    ) -> ProgramResult {
        assert!(fee < 1_000_000); // 100%

        // TODO examine max value of tick_spacing
        // tick spacing is capped at 16384 to prevent the situation where tick_spacing is so large that
        // tick_bitmap#next_initialized_tick_within_one_word overflows int24 container from a valid tick
        // 16384 ticks represents a >5x price change with ticks of 1 bips
        let mut fee_state = ctx.accounts.fee_state.load_init()?;
        assert!(tick_spacing > 0 && tick_spacing < 16384);
        fee_state.bump = fee_state_bump;
        fee_state.fee = fee;
        fee_state.tick_spacing = tick_spacing;

        emit!(FeeAmountEnabled { fee, tick_spacing });
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Pool instructions

    /// Creates a pool for the given token pair and fee, and sets the initial price
    ///
    /// A single function in place of Uniswap's Factory.createPool(), PoolDeployer.deploy()
    /// Pool.initialize() and pool.Constructor()
    ///
    /// # Arguments
    ///
    /// * `ctx`- Validates token addresses and fee state. Initializes pool, observation and
    /// token accounts
    /// * `pool_state_bump` - Bump to validate Pool State address
    /// * `observation_state_bump` - Bump to validate Observation State address
    /// * `sqrt_price_x32` - the initial sqrt price (amount_token_1 / amount_token_0) of the pool as a Q32.32
    ///
    pub fn create_and_init_pool(
        ctx: Context<CreateAndInitPool>,
        pool_state_bump: u8,
        observation_state_bump: u8,
        sqrt_price_x32: u64,
    ) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_init()?;
        let mut initial_observation_state = ctx.accounts.initial_observation_state.load_init()?;
        let fee_state = ctx.accounts.fee_state.load()?;
        let tick = tick_math::get_tick_at_sqrt_ratio(sqrt_price_x32)?;

        pool_state.bump = pool_state_bump;
        pool_state.token_0 = ctx.accounts.token_0.key();
        pool_state.token_1 = ctx.accounts.token_1.key();
        pool_state.fee = fee_state.fee;
        pool_state.tick_spacing = fee_state.tick_spacing;
        pool_state.sqrt_price_x32 = sqrt_price_x32;
        pool_state.tick = tick;
        pool_state.unlocked = true;
        pool_state.observation_cardinality = 1;
        pool_state.observation_cardinality_next = 1;

        initial_observation_state.bump = observation_state_bump;
        initial_observation_state.block_timestamp = oracle::_block_timestamp();
        initial_observation_state.initialized = true;

        // default value 0 for remaining variables

        emit!(PoolCreatedAndInitialized {
            token_0: ctx.accounts.token_0.key(),
            token_1: ctx.accounts.token_1.key(),
            fee: fee_state.fee,
            tick_spacing: fee_state.tick_spacing,
            pool_state: ctx.accounts.pool_state.key(),
            sqrt_price_x32,
            tick,
        });
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Oracle

    /// Increase the maximum number of price and liquidity observations that this pool will store
    ///
    /// An `ObservationState` account is created per unit increase in cardinality_next,
    /// and `observation_cardinality_next` is accordingly incremented.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds the pool and payer addresses, along with a vector of
    /// observation accounts which will be initialized
    /// * `observation_account_bumps` - Vector of bumps to initialize the observation state PDAs
    ///
    pub fn increase_observation_cardinality_next<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, IncreaseObservationCardinalityNext<'info>>,
        observation_account_bumps: Vec<u8>,
    ) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        let mut i: usize = 0;
        while i < observation_account_bumps.len() {
            let observation_account_seeds = [
                &OBSERVATION_SEED.as_bytes(),
                pool_state.token_0.as_ref(),
                pool_state.token_1.as_ref(),
                &pool_state.fee.to_be_bytes(),
                &(pool_state.observation_cardinality_next + i as u16).to_be_bytes(),
                &[observation_account_bumps[i]],
            ];

            require!(
                ctx.remaining_accounts[i].key()
                    == Pubkey::create_program_address(
                        &observation_account_seeds[..],
                        &ctx.program_id
                    )?,
                ErrorCode::OS
            );

            let space = 8 + size_of::<ObservationState>();
            let rent = Rent::get()?;
            let lamports = rent.minimum_balance(space);
            let ix = create_account(
                ctx.accounts.payer.key,
                &ctx.remaining_accounts[i].key,
                lamports,
                space as u64,
                ctx.program_id,
            );

            solana_program::program::invoke_signed(
                &ix,
                &[
                    ctx.accounts.payer.to_account_info(),
                    ctx.remaining_accounts[i].to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[&observation_account_seeds[..]],
            )?;

            let observation_state_loader = Loader::<ObservationState>::try_from_unchecked(
                &cyclos_core::id(),
                &ctx.remaining_accounts[i].to_account_info(),
            )?;
            let mut observation_state = observation_state_loader.load_init()?;
            // this data will not be used because the initialized boolean is still false
            observation_state.bump = observation_account_bumps[i];
            observation_state.index = pool_state.observation_cardinality_next + i as u16;
            observation_state.block_timestamp = 1;

            drop(observation_state);
            observation_state_loader.exit(ctx.program_id)?;

            i += 1;
        }
        let observation_cardinality_next_old = pool_state.observation_cardinality_next;
        pool_state.observation_cardinality_next += i as u16;

        emit!(oracle::IncreaseObservationCardinalityNext {
            observation_cardinality_next_old,
            observation_cardinality_next_new: pool_state.observation_cardinality_next,
        });

        pool_state.unlocked = true;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Pool owner instructions

    /// Set the denominator of the protocol's % share of the fees
    ///
    /// # Arguments
    ///
    /// * `ctx` - Checks for valid owner by looking at signer and factory owner addresses.
    /// Holds the Pool State account where protocol fee will be saved.
    /// * `fee_protocol_0` - new protocol fee for token_0 of the pool
    /// * `fee_protocol_1` - new protocol fee for token_1 of the pool
    ///
    pub fn set_fee_protocol(
        ctx: Context<SetFeeProtocol>,
        fee_protocol_0: u8,
        fee_protocol_1: u8,
    ) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        assert!(
            (fee_protocol_0 == 0 || (fee_protocol_0 >= 4 && fee_protocol_0 <= 10))
                && (fee_protocol_1 == 0 || (fee_protocol_1 >= 4 && fee_protocol_1 <= 10))
        );

        let fee_protocol_old = pool_state.fee_protocol;
        pool_state.fee_protocol = (fee_protocol_1 << 4) + fee_protocol_0;

        emit!(SetFeeProtocolEvent {
            pool_state: ctx.accounts.pool_state.key(),
            fee_protocol_0_old: fee_protocol_old % 16,
            fee_protocol_1_old: fee_protocol_old >> 4,
            fee_protocol_0,
            fee_protocol_1,
        });

        pool_state.unlocked = true;
        Ok(())
    }

    /// Collect the protocol fee accrued to the pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - Checks for valid owner by looking at signer and factory owner addresses.
    /// Holds the Pool State account where accrued protocol fee is saved, and token accounts to perform
    /// transfer.
    /// * `amount_0_requested` - The maximum amount of token_0 to send, can be 0 to collect fees in only token_1
    /// * `amount_1_requested` - The maximum amount of token_1 to send, can be 0 to collect fees in only token_0
    ///
    pub fn collect_protocol(
        ctx: Context<CollectProtocol>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        let amount_0 = amount_0_requested.min(pool_state.protocol_fees_token_0);
        let amount_1 = amount_1_requested.min(pool_state.protocol_fees_token_1);

        let pool_state_seeds = [
            &POOL_SEED.as_bytes(),
            &pool_state.token_0.to_bytes() as &[u8],
            &pool_state.token_1.to_bytes() as &[u8],
            &pool_state.fee.to_be_bytes(),
            &[pool_state.bump],
        ];

        if amount_0 > 0 {
            pool_state.protocol_fees_token_0 -= amount_0;
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_0.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_0.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]],
                ),
                amount_0,
            )?;
        }
        if amount_1 > 0 {
            pool_state.protocol_fees_token_1 -= amount_1;
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_1.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_1.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]],
                ),
                amount_1,
            )?;
        }

        emit!(CollectProtocolEvent {
            pool_state: ctx.accounts.pool_state.key(),
            sender: ctx.accounts.owner.key(),
            recipient_wallet_0: ctx.accounts.recipient_wallet_0.key(),
            recipient_wallet_1: ctx.accounts.recipient_wallet_1.key(),
            amount_0,
            amount_1,
        });

        pool_state.unlocked = true;
        Ok(())
    }

    /// ---------------------------------------------------------------------
    /// Account init instructions
    ///
    /// Having separate instructions to initialize instructions saves compute units
    /// and reduces code in downstream instructions
    ///

    /// Initializes an empty program account for a price tick
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains accounts to initialize an empty tick account
    /// * `tick_account_bump` - Bump to validate tick account PDA
    /// * `tick` - The tick for which the account is created
    ///
    pub fn init_tick_account(
        ctx: Context<InitTickAccount>,
        tick_account_bump: u8,
        tick: i32,
    ) -> ProgramResult {
        let pool_state = ctx.accounts.pool_state.load()?;
        check_tick(tick, pool_state.tick_spacing)?;
        let mut tick_account = ctx.accounts.tick_state.load_init()?;
        tick_account.bump = tick_account_bump;
        tick_account.tick = tick;
        Ok(())
    }

    /// Initializes an empty program account for a tick bitmap
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains accounts to initialize an empty bitmap account
    /// * `bitmap_account_bump` - Bump to validate the bitmap account PDA
    /// * `word_pos` - The bitmap key for which to create account. To find word position from a tick,
    /// divide the tick by tick spacing to get a 24 bit compressed result, then right shift to obtain the
    /// most significant 16 bits.
    ///
    pub fn init_bitmap_account(
        ctx: Context<InitBitmapAccount>,
        bump: u8,
        word_pos: i16,
    ) -> ProgramResult {
        let pool_state = ctx.accounts.pool_state.load()?;
        let max_word_pos = ((tick_math::MAX_TICK / pool_state.tick_spacing as i32) >> 8) as i16;
        let min_word_pos = ((tick_math::MIN_TICK / pool_state.tick_spacing as i32) >> 8) as i16;
        require!(word_pos >= min_word_pos, ErrorCode::TLM);
        require!(word_pos <= max_word_pos, ErrorCode::TUM);

        let mut bitmap_account = ctx.accounts.bitmap_state.load_init()?;
        bitmap_account.bump = bump;
        bitmap_account.word_pos = word_pos;
        Ok(())
    }

    /// Initializes an empty program account for a position
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains accounts to initialize an empty position account
    /// * `bump` - Bump to validate the position account PDA
    /// * `tick` - The tick for which the bitmap account is created. Program address of
    /// the account is derived using most significant 16 bits of the tick
    ///
    pub fn init_position_account(ctx: Context<InitPositionAccount>, bump: u8) -> ProgramResult {
        let mut position_account = ctx.accounts.position_state.load_init()?;
        position_account.bump = bump;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Position instructions

    /// Callback to pay tokens for creating or adding liquidity to a position
    ///
    /// Callback function lies in core program instead of non_fungible_position_manager since
    /// reentrancy is disallowed in Solana. Integrators can use a second program to handle callbacks.
    ///
    /// # Arguments
    ///
    /// * `amount_0_owed` - The amount of token_0 due to the pool for the minted liquidity
    /// * `amount_1_owed` - The amount of token_1 due to the pool for the minted liquidity
    ///
    pub fn mint_callback(
        ctx: Context<MintCallback>,
        amount_0_owed: u64,
        amount_1_owed: u64,
    ) -> ProgramResult {
        if amount_0_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.token_account_0.to_account_info(),
                        to: ctx.accounts.vault_0.to_account_info(),
                        authority: ctx.accounts.minter.to_account_info(),
                    },
                ),
                amount_0_owed,
            )?;
        }
        if amount_1_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.token_account_1.to_account_info(),
                        to: ctx.accounts.vault_1.to_account_info(),
                        authority: ctx.accounts.minter.to_account_info(),
                    },
                ),
                amount_1_owed,
            )?;
        }
        Ok(())
    }

    /// Callback to pay the pool tokens owed for the swap.
    /// The caller of this method must be checked to be the core program.
    /// amount_0_delta and amount_1_delta can both be 0 if no tokens were swapped.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Token accounts for payment
    /// * `exact_input` - Whether an exact input swap or an exact output swap
    /// * `amount_0_delta` - The amount of token_0 that was sent (negative) or must be received (positive) by the pool by
    /// the end of the swap. If positive, the callback must send that amount of token_0 to the pool.
    /// * `amount_1_delta` - The amount of token_1 that was sent (negative) or must be received (positive) by the pool by
    /// the end of the swap. If positive, the callback must send that amount of token_1 to the pool.
    ///
    pub fn swap_callback(
        ctx: Context<SwapCallback>,
        exact_input: bool,
        amount_0_delta: i64,
        amount_1_delta: i64,
    ) -> ProgramResult {
        // TODO check caller address and implement for exact output swaps
        assert!(amount_0_delta > 0 || amount_1_delta > 0); // swaps entirely within 0-liquidity regions are not supported

        assert!(exact_input); // exact output not yet implemented
        if amount_0_delta > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.token_account_0.to_account_info(),
                        to: ctx.accounts.vault_0.to_account_info(),
                        authority: ctx.accounts.signer.to_account_info(),
                    },
                ),
                amount_0_delta as u64,
            )?;
        } else {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.token_account_1.to_account_info(),
                        to: ctx.accounts.vault_1.to_account_info(),
                        authority: ctx.accounts.signer.to_account_info(),
                    },
                ),
                amount_1_delta as u64,
            )?;
        }

        Ok(())
    }

    /// Adds liquidity for the given pool/recipient/tickLower/tickUpper position
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds the recipient's address and program accounts for
    /// pool, position and ticks.
    /// * `amount` - The amount of liquidity to mint
    ///
    pub fn mint(ctx: Context<MintContext>, amount: u64) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        assert!(amount > 0);

        let (amount_0_int, amount_1_int) = _modify_position(
            pool_state.deref_mut(),
            &ctx.accounts.position_state,
            &ctx.accounts.tick_lower_state,
            &ctx.accounts.tick_upper_state,
            &ctx.accounts.bitmap_lower,
            &ctx.accounts.bitmap_upper,
            &ctx.accounts.latest_observation_state,
            &ctx.accounts.next_observation_state,
            ctx.accounts.minter.to_account_info(),
            i64::try_from(amount).unwrap(),
        )?;

        let amount_0 = amount_0_int as u64;
        let amount_1 = amount_1_int as u64;

        let balance_0_before = if amount_0 > 0 {
            ctx.accounts.token_account_0.amount
        } else {
            0
        };
        let balance_1_before = if amount_0 > 0 {
            ctx.accounts.token_account_1.amount
        } else {
            0
        };

        drop(pool_state);

        let mint_callback_ix = cyclos_core::instruction::MintCallback {
            amount_0_owed: amount_0,
            amount_1_owed: amount_1,
        };
        let ix = Instruction::new_with_bytes(
            ctx.accounts.callback_handler.key(),
            &mint_callback_ix.data(),
            ctx.accounts.to_account_metas(None),
        );
        solana_program::program::invoke(&ix, &ctx.accounts.to_account_infos())?;

        ctx.accounts.token_account_0.reload()?;
        ctx.accounts.token_account_1.reload()?;

        if amount_0 > 0 {
            require!(
                balance_0_before + amount_0 <= ctx.accounts.token_account_0.amount,
                ErrorCode::M0
            );
        }
        if amount_1 > 0 {
            require!(
                balance_1_before + amount_1 <= ctx.accounts.token_account_1.amount,
                ErrorCode::M1
            );
        }

        emit!(MintEvent {
            pool_state: ctx.accounts.pool_state.key(),
            sender: ctx.accounts.minter.key(),
            owner: ctx.accounts.recipient.key(),
            tick_lower: ctx.accounts.tick_lower_state.load()?.tick,
            tick_upper: ctx.accounts.tick_upper_state.load()?.tick,
            amount,
            amount_0,
            amount_1
        });

        ctx.accounts.pool_state.load_mut()?.unlocked = true;
        Ok(())
    }

    /// Burn liquidity from the sender and account tokens owed for the liquidity to the position.
    /// Can be used to trigger a recalculation of fees owed to a position by calling with an amount of 0 (poke).
    /// Fees must be collected separately via a call to #collect
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds position and other validated accounts need to burn liquidity
    /// * `amount` - Amount of liquidity to be burned
    ///
    pub fn burn(ctx: Context<BurnContext>, amount: u64) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        let (amount_0_int, amount_1_int) = _modify_position(
            pool_state.deref_mut(),
            &ctx.accounts.position_state,
            &ctx.accounts.tick_lower_state,
            &ctx.accounts.tick_upper_state,
            &ctx.accounts.bitmap_lower,
            &ctx.accounts.bitmap_upper,
            &ctx.accounts.latest_observation_state,
            &ctx.accounts.next_observation_state,
            ctx.accounts.lamport_destination.to_account_info(),
            -i64::try_from(amount).unwrap(),
        )?;

        let amount_0 = (-amount_0_int) as u64;
        let amount_1 = (-amount_1_int) as u64;
        if amount_0 > 0 || amount_1 > 0 {
            let mut position_state = ctx.accounts.position_state.load_mut()?;
            position_state.tokens_owed_0 += amount_0;
            position_state.tokens_owed_1 += amount_1;
        }

        emit!(BurnEvent {
            pool_state: ctx.accounts.pool_state.key(),
            owner: ctx.accounts.owner.key(),
            tick_lower: ctx.accounts.tick_lower_state.load()?.tick,
            tick_upper: ctx.accounts.tick_upper_state.load()?.tick,
            amount,
            amount_0,
            amount_1,
        });

        pool_state.unlocked = true;
        Ok(())
    }

    /// Collects tokens owed to a position.
    ///
    /// Does not recompute fees earned, which must be done either via mint or burn of any amount of liquidity.
    /// Collect must be called by the position owner. To withdraw only token_0 or only token_1, amount_0_requested or
    /// amount_1_requested may be set to zero. To withdraw all tokens owed, caller may pass any value greater than the
    /// actual tokens owed, e.g. u64::MAX. Tokens owed may be from accumulated swap fees or burned liquidity.
    ///
    /// # Arguments
    ///
    /// * `amount_0_requested` - How much token_0 should be withdrawn from the fees owed
    /// * `amount_1_requested` - How much token_1 should be withdrawn from the fees owed
    ///
    pub fn collect(
        ctx: Context<CollectContext>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> ProgramResult {
        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        pool_state.unlocked = false;

        let mut position = ctx.accounts.position_state.load_mut()?;

        let amount_0 = amount_0_requested.min(position.tokens_owed_0);
        let amount_1 = amount_1_requested.min(position.tokens_owed_1);

        let pool_state_seeds = [
            &POOL_SEED.as_bytes(),
            &pool_state.token_0.to_bytes() as &[u8],
            &pool_state.token_1.to_bytes() as &[u8],
            &pool_state.fee.to_be_bytes(),
            &[pool_state.bump],
        ];

        drop(pool_state);
        if amount_0 > 0 {
            position.tokens_owed_0 -= amount_0;
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_0.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_0.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]],
                ),
                amount_0,
            )?;
        }
        if amount_1 > 0 {
            position.tokens_owed_1 -= amount_1;
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_1.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_1.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]],
                ),
                amount_1,
            )?;
        }

        emit!(CollectEvent {
            pool_state: ctx.accounts.pool_state.key(),
            owner: ctx.accounts.owner.key(),
            tick_lower: ctx.accounts.tick_lower_state.load()?.tick,
            tick_upper: ctx.accounts.tick_upper_state.load()?.tick,
            amount_0,
            amount_1,
        });

        ctx.accounts.pool_state.load_mut()?.unlocked = true;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // 4. Swap instructions

    pub struct SwapCache {
        // the protocol fee for the input token
        pub fee_protocol: u8,
        // liquidity at the beginning of the swap
        pub liquidity_start: u64,
        // the timestamp of the current block
        pub block_timestamp: u32,
        // the current value of the tick accumulator, computed only if we cross an initialized tick
        pub tick_cumulative: i64,
        // the current value of seconds per liquidity accumulator, computed only if we cross an initialized tick
        pub seconds_per_liquidity_cumulative_x32: u64,
        // whether we've computed and cached the above two accumulators
        pub computed_latest_observation: bool,
    }

    // the top level state of the swap, the results of which are recorded in storage at the end
    pub struct SwapState {
        // the amount remaining to be swapped in/out of the input/output asset
        pub amount_specified_remaining: i64,
        // the amount already swapped out/in of the output/input asset
        pub amount_calculated: i64,
        // current sqrt(price)
        pub sqrt_price_x32: u64,
        // the tick associated with the current price
        pub tick: i32,
        // the global fee growth of the input token
        pub fee_growth_global_x32: u64,
        // amount of input token paid as protocol fee
        pub protocol_fee: u64,
        // the current liquidity in range
        pub liquidity: u64,
    }

    #[derive(Default)]
    struct StepComputations {
        // the price at the beginning of the step
        sqrt_price_start_x32: u64,
        // the next tick to swap to from the current tick in the swap direction
        tick_next: i32,
        // whether tick_next is initialized or not
        initialized: bool,
        // sqrt(price) for the next tick (1/0)
        sqrt_price_next_x32: u64,
        // how much is being swapped in in this step
        amount_in: u64,
        // how much is being swapped out
        amount_out: u64,
        // how much fee is being paid in
        fee_amount: u64,
    }

    /// Swap token_0 for token_1, or token_1 for token_0
    ///
    /// Outstanding tokens must be paid in #swap_callback
    ///
    /// # Arguments
    ///
    /// * `ctx` - Accounts required for the swap
    /// * `zero_for_one` -  The direction of the swap, true for token_0 to token_1, false for token_1 to token_0
    /// * `deadline` - The time by which the transaction must be included to effect the change
    /// * `amount_specified` - The amount of the swap, which implicitly configures the swap as exact input (positive),
    /// or exact output (negative)
    /// * `sqrt_price_limit` - The Q32.32 sqrt price √P limit. If zero for one, the price cannot
    /// be less than this value after the swap.  If one for zero, the price cannot be greater than
    /// this value after the swap.
    ///
    pub fn swap(
        ctx: Context<SwapContext>,
        zero_for_one: bool,
        amount_specified: i64,
        sqrt_price_limit_x32: u64,
    ) -> ProgramResult {
        msg!("in core swap");
        require!(amount_specified != 0, ErrorCode::AS);

        let mut pool_state = ctx.accounts.pool_state.load_mut()?;
        require!(pool_state.unlocked, ErrorCode::LOK);
        require!(
            if zero_for_one {
                sqrt_price_limit_x32 < pool_state.sqrt_price_x32
                    && sqrt_price_limit_x32 > tick_math::MIN_SQRT_RATIO
            } else {
                sqrt_price_limit_x32 > pool_state.sqrt_price_x32
                    && sqrt_price_limit_x32 < tick_math::MAX_SQRT_RATIO
            },
            ErrorCode::SPL
        );

        pool_state.unlocked = false;

        let mut cache = SwapCache {
            liquidity_start: pool_state.liquidity,
            block_timestamp: oracle::_block_timestamp(),
            fee_protocol: if zero_for_one {
                pool_state.fee_protocol % 16
            } else {
                pool_state.fee_protocol >> 4
            },
            seconds_per_liquidity_cumulative_x32: 0,
            tick_cumulative: 0,
            computed_latest_observation: false,
        };

        let exact_input = amount_specified > 0;

        let mut state = SwapState {
            amount_specified_remaining: amount_specified,
            amount_calculated: 0,
            sqrt_price_x32: pool_state.sqrt_price_x32,
            tick: pool_state.tick,
            fee_growth_global_x32: if zero_for_one {
                pool_state.fee_growth_global_0_x32
            } else {
                pool_state.fee_growth_global_1_x32
            },
            protocol_fee: 0,
            liquidity: cache.liquidity_start,
        };

        // continue swapping as long as we haven't used the entire input/output and haven't
        // reached the price limit
        let latest_observation = ctx.accounts.latest_observation_state.load_mut()?;
        let mut remaining_accounts = ctx.remaining_accounts.iter();
        let mut bitmap: Option<TickBitmapState> = None;
        while state.amount_specified_remaining != 0 && state.sqrt_price_x32 != sqrt_price_limit_x32
        {
            let mut step = StepComputations::default();
            step.sqrt_price_start_x32 = state.sqrt_price_x32;

            // if zero_for_one(lte = true) start with the word containing the current tick
            // else use the word of the next tick
            // what about looping?
            let mut compressed = state.tick / pool_state.tick_spacing as i32;
            if state.tick < 0 && state.tick % pool_state.tick_spacing as i32 != 0 {
                compressed -= 1;
            }
            if !zero_for_one {
                compressed += 1;
            }

            let Position { word_pos, bit_pos } = tick_bitmap::position(compressed);

            if bitmap.is_none() || bitmap.unwrap().word_pos != word_pos {
                let bitmap_loader = Loader::<TickBitmapState>::try_from(
                    &cyclos_core::id(),
                    remaining_accounts.next().unwrap(),
                )?;
                let bitmap_state = bitmap_loader.load()?;

                let bitmap_account_seeds = [
                    BITMAP_SEED.as_bytes(),
                    pool_state.token_0.as_ref(),
                    pool_state.token_1.as_ref(),
                    &pool_state.fee.to_be_bytes(),
                    &word_pos.to_be_bytes(),
                    &[bitmap_state.bump],
                ];
                assert!(
                    bitmap_loader.key()
                        == Pubkey::create_program_address(
                            &bitmap_account_seeds[..],
                            &ctx.program_id
                        )?,
                );
                bitmap = Some(*bitmap_state.deref());
                drop(bitmap_state);
            }

            let next_initialized_bit = bitmap.unwrap().next_initialized_bit(bit_pos, zero_for_one);
            step.tick_next =
                (compressed + next_initialized_bit.next) * pool_state.tick_spacing as i32;
            step.initialized = next_initialized_bit.initialized;

            // ensure that we do not overshoot the min/max tick, as the tick bitmap is not aware of these bounds
            if step.tick_next < tick_math::MIN_TICK {
                step.tick_next = tick_math::MIN_TICK;
            } else if step.tick_next > tick_math::MAX_TICK {
                step.tick_next = tick_math::MAX_TICK;
            }

            step.sqrt_price_next_x32 = tick_math::get_sqrt_ratio_at_tick(step.tick_next)?;

            let swap_step = swap_math::compute_swap_step(
                state.sqrt_price_x32,
                if (zero_for_one && step.sqrt_price_next_x32 < sqrt_price_limit_x32)
                    || (!zero_for_one && step.sqrt_price_next_x32 > sqrt_price_limit_x32)
                {
                    sqrt_price_limit_x32
                } else {
                    step.sqrt_price_next_x32
                },
                state.liquidity,
                state.amount_specified_remaining,
                pool_state.fee,
            );
            state.sqrt_price_x32 = swap_step.sqrt_ratio_next_x32;
            step.amount_in = swap_step.amount_in;
            step.amount_out = swap_step.amount_out;
            step.fee_amount = swap_step.fee_amount;

            if exact_input {
                state.amount_specified_remaining -=
                    i64::try_from(step.amount_in + step.fee_amount).unwrap();
                state.amount_calculated = state
                    .amount_calculated
                    .checked_sub(i64::try_from(step.amount_out).unwrap())
                    .unwrap();
            } else {
                state.amount_specified_remaining += i64::try_from(step.amount_out).unwrap();
                state.amount_calculated = state
                    .amount_calculated
                    .checked_add(i64::try_from(step.amount_in + step.fee_amount).unwrap())
                    .unwrap();
            }

            // if the protocol fee is on, calculate how much is owed, decrement fee_amount, and increment protocol_fee
            if cache.fee_protocol > 0 {
                let delta = step.fee_amount / cache.fee_protocol as u64;
                step.fee_amount -= delta;
                state.protocol_fee += delta;
            }

            // update global fee tracker
            if state.liquidity > 0 {
                state.fee_growth_global_x32 += step
                    .fee_amount
                    .mul_div_floor(fixed_point_x32::Q32, state.liquidity)
                    .unwrap();
            }

            // shift tick if we reached the next price
            if state.sqrt_price_x32 == step.sqrt_price_next_x32 {
                // if the tick is initialized, run the tick transition
                if step.initialized {
                    // check for the placeholder value for the oracle observation, which we replace with the
                    // actual value the first time the swap crosses an initialized tick
                    if !cache.computed_latest_observation {
                        let new_observation = latest_observation.observe_latest(
                            cache.block_timestamp,
                            pool_state.tick,
                            pool_state.liquidity,
                        );
                        cache.tick_cumulative = new_observation.0;
                        cache.seconds_per_liquidity_cumulative_x32 = new_observation.1;
                        cache.computed_latest_observation = true;
                    }

                    let tick_loader = Loader::<TickState>::try_from(
                        &cyclos_core::id(),
                        remaining_accounts.next().unwrap(),
                    )?;
                    let mut tick_state = tick_loader.load_mut()?;
                    let tick_account_seeds = [
                        &TICK_SEED.as_bytes(),
                        pool_state.token_0.as_ref(),
                        pool_state.token_1.as_ref(),
                        &pool_state.fee.to_be_bytes(),
                        &step.tick_next.to_be_bytes(),
                        &[tick_state.bump],
                    ];
                    assert!(
                        tick_loader.key()
                            == Pubkey::create_program_address(
                                &tick_account_seeds[..],
                                &ctx.program_id
                            )?,
                    );
                    let mut liquidity_net = tick_state.deref_mut().cross(
                        if zero_for_one {
                            state.fee_growth_global_x32
                        } else {
                            pool_state.fee_growth_global_0_x32
                        },
                        if zero_for_one {
                            pool_state.fee_growth_global_1_x32
                        } else {
                            state.fee_growth_global_x32
                        },
                        cache.seconds_per_liquidity_cumulative_x32,
                        cache.tick_cumulative,
                        cache.block_timestamp,
                    );

                    // if we're moving leftward, we interpret liquidity_net as the opposite sign
                    // safe because liquidity_net cannot be i64::MIN
                    if zero_for_one {
                        liquidity_net = liquidity_net.neg();
                    }

                    state.liquidity = liquidity_math::add_delta(state.liquidity, liquidity_net)?;
                }

                state.tick = if zero_for_one {
                    step.tick_next - 1
                } else {
                    step.tick_next
                };
            } else if state.sqrt_price_x32 != step.sqrt_price_start_x32 {
                // recompute unless we're on a lower tick boundary (i.e. already transitioned ticks), and haven't moved
                state.tick = tick_math::get_tick_at_sqrt_ratio(state.sqrt_price_x32)?;
            }
        }
        let next_observation_start_time = (latest_observation.block_timestamp / 14 + 1) * 14;
        drop(latest_observation);
        // update tick and write an oracle entry if the tick changes
        if state.tick != pool_state.tick {

            let mut next_observation = if cache.block_timestamp >= next_observation_start_time {
                let next_observation = ctx.accounts.next_observation_state.load_mut()?;
                pool_state.observation_index = next_observation.index;
                next_observation
            } else {
                ctx.accounts.latest_observation_state.load_mut()?
                // latest_observation
            };
            pool_state.tick = state.tick;
            pool_state.observation_cardinality_next = next_observation.update(
                cache.block_timestamp,
                pool_state.tick,
                cache.liquidity_start,
                pool_state.observation_cardinality,
                pool_state.observation_cardinality_next,
            );
        }
        // drop(latest_observation);
        pool_state.sqrt_price_x32 = state.sqrt_price_x32;

        // update liquidity if it changed
        if cache.liquidity_start != state.liquidity {
            pool_state.liquidity = state.liquidity;
        }

        // update fee growth global and, if necessary, protocol fees
        // overflow is acceptable, protocol has to withdraw before it hit u64::MAX fees
        if zero_for_one {
            pool_state.fee_growth_global_0_x32 = state.fee_growth_global_x32;
            if state.protocol_fee > 0 {
                pool_state.protocol_fees_token_0 += state.protocol_fee;
            }
        } else {
            pool_state.fee_growth_global_1_x32 = state.fee_growth_global_x32;
            if state.protocol_fee > 0 {
                pool_state.protocol_fees_token_1 += state.protocol_fee;
            }
        }

        let (amount_0, amount_1) = if zero_for_one == exact_input {
            (
                amount_specified - state.amount_specified_remaining,
                state.amount_calculated,
            )
        } else {
            (
                state.amount_calculated,
                amount_specified - state.amount_specified_remaining,
            )
        };

        // do the transfers and collect payment
        let pool_state_seeds = [
            &POOL_SEED.as_bytes(),
            &pool_state.token_0.to_bytes() as &[u8],
            &pool_state.token_1.to_bytes() as &[u8],
            &pool_state.fee.to_be_bytes(),
            &[pool_state.bump],
        ];
        drop(pool_state);

        if zero_for_one {
            if amount_1 < 0 {
                token::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info().clone(),
                        token::Transfer {
                            from: ctx.accounts.vault_1.to_account_info().clone(),
                            to: ctx.accounts.token_account_1.to_account_info().clone(),
                            authority: ctx.accounts.pool_state.to_account_info().clone(),
                        },
                        &[&pool_state_seeds[..]],
                    ),
                    amount_1.neg() as u64,
                )?;
            }
            let balance_0_before = ctx.accounts.vault_0.amount;
            // transfer tokens to pool in callback
            let swap_callback_ix = cyclos_core::instruction::SwapCallback {
                amount_0_delta: amount_0,
                amount_1_delta: amount_1,
                exact_input,
            };
            let ix = Instruction::new_with_bytes(
                ctx.accounts.callback_handler.key(),
                &swap_callback_ix.data(),
                ctx.accounts.to_account_metas(None),
            );
            solana_program::program::invoke(
                &ix,
                &ctx.accounts.to_account_infos(),
            )?;
            ctx.accounts.vault_0.reload()?;
            require!(
                balance_0_before.checked_add(amount_0 as u64).unwrap() <= ctx.accounts.vault_0.amount,
                ErrorCode::IIA
            );
        } else {
            if amount_0 < 0 {
                token::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info().clone(),
                        token::Transfer {
                            from: ctx.accounts.vault_0.to_account_info().clone(),
                            to: ctx.accounts.token_account_0.to_account_info().clone(),
                            authority: ctx.accounts.pool_state.to_account_info().clone(),
                        },
                        &[&pool_state_seeds[..]],
                    ),
                    amount_0.neg() as u64,
                )?;
            }
            let balance_1_before = ctx.accounts.vault_1.amount;
            // transfer tokens to pool in callback
            let swap_callback_ix = cyclos_core::instruction::SwapCallback {
                amount_0_delta: amount_0,
                amount_1_delta: amount_1,
                exact_input,
            };
            let ix = Instruction::new_with_bytes(
                ctx.accounts.callback_handler.key(),
                &swap_callback_ix.data(),
                ctx.accounts.to_account_metas(None),
            );
            solana_program::program::invoke(
                &ix,
                &ctx.accounts.to_account_infos(),
            )?;
            ctx.accounts.vault_1.reload()?;
            require!(
                balance_1_before.checked_add(amount_0 as u64).unwrap() <= ctx.accounts.vault_1.amount,
                ErrorCode::IIA
            );
        }

        emit!(SwapEvent {
            pool_state: ctx.accounts.pool_state.key(),
            sender: ctx.accounts.signer.key(),
            token_account_0: ctx.accounts.token_account_0.key(),
            token_account_1: ctx.accounts.token_account_1.key(),
            amount_0,
            amount_1,
            sqrt_price_x32: state.sqrt_price_x32,
            liquidity: state.liquidity,
            tick: state.tick
        });
        ctx.accounts.pool_state.load_mut()?.unlocked = true;
        Ok(())
    }

    // /// Component function for flash swaps
    // ///
    // /// Donate given liquidity to in-range positions then make callback
    // /// Only callable by a smart contract which implements uniswapV3FlashCallback(),
    // /// where profitability check can be performed
    // ///
    // /// Flash swaps is an advanced feature for developers, not directly available for UI based traders.
    // /// Periphery does not provide an implementation, but a sample is provided
    // /// Ref- https://github.com/Uniswap/v3-periphery/blob/main/contracts/examples/PairFlash.sol
    // ///
    // ///
    // /// Flow
    // /// 1. FlashDapp.initFlash()
    // /// 2. Core.flash()
    // /// 3. FlashDapp.uniswapV3FlashCallback()
    // ///
    // /// @param amount_0 Amount of token 0 to donate
    // /// @param amount_1 Amount of token 1 to donate
    // pub fn flash(ctx: Context<SetFeeProtocol>, amount_0: u64, amount_1: u64) -> ProgramResult {
    //     todo!()
    // }
}

/// Common checks for a valid tick input.
/// A tick is valid iff it lies within tick boundaries and it is a multiple
/// of tick spacing.
///
/// # Arguments
///
/// * `tick` - The price tick
///
pub fn check_tick(tick: i32, tick_spacing: u16) -> Result<(), ErrorCode> {
    require!(tick >= tick_math::MIN_TICK, ErrorCode::TLM);
    require!(tick <= tick_math::MAX_TICK, ErrorCode::TUM);
    require!(tick % tick_spacing as i32 == 0, ErrorCode::TMS);
    Ok(())
}

/// Common checks for valid tick inputs.
///
/// # Arguments
///
/// * `tick_lower` - The lower tick
/// * `tick_upper` - The upper tick
///
pub fn check_ticks(tick_lower: i32, tick_upper: i32) -> Result<(), ErrorCode> {
    require!(tick_lower < tick_upper, ErrorCode::TLU);
    Ok(())
}

/// Credit or debit liquidity to a position, and find the amount of token_0 and token_1
/// required to produce this change.
/// Returns amount of token_0 and token_1 owed to the pool, negative if the pool should
/// pay the recipient.
///
/// # Arguments
///
/// * `position_state` - Effect change to this position
/// * `tick_lower_state`- Program account for the lower tick boundary
/// * `tick_upper_state`- Program account for the upper tick boundary
/// * `bitmap_lower` - Holds the initialization state of the lower tick
/// * `bitmap_upper` - Holds the initialization state of the upper tick
/// * `latest_observation_state` - Most recent oracle observation
/// * `next_observation_state` - Account to store the next oracle observation
/// * `lamport_destination` - Destination account for freed lamports when a tick state is
/// un-initialized
/// * `liquidity_delta` - The change in liquidity. Can be 0 to perform a poke.
///
pub fn _modify_position<'info>(
    pool_state: &mut PoolState,
    position_state: &Loader<'info, PositionState>,
    tick_lower_state: &Loader<'info, TickState>,
    tick_upper_state: &Loader<'info, TickState>,
    bitmap_lower: &Loader<'info, TickBitmapState>,
    bitmap_upper: &Loader<'info, TickBitmapState>,
    latest_observation_state: &Loader<'info, ObservationState>,
    next_observation_state: &Loader<'info, ObservationState>,
    lamport_destination: AccountInfo<'info>,
    liquidity_delta: i64,
) -> Result<(i64, i64), ProgramError> {
    check_ticks(tick_lower_state.load()?.tick, tick_upper_state.load()?.tick)?;

    let latest_observation = latest_observation_state.load_mut()?;

    _update_position(
        pool_state.deref(),
        position_state,
        tick_lower_state,
        tick_upper_state,
        bitmap_lower,
        bitmap_upper,
        latest_observation.deref(),
        lamport_destination,
        liquidity_delta,
    )?;

    let mut amount_0 = 0;
    let mut amount_1 = 0;

    let tick_lower = tick_lower_state.load()?.tick;
    let tick_upper = tick_upper_state.load()?.tick;

    if liquidity_delta != 0 {
        if pool_state.tick < tick_lower {
            // current tick is below the passed range; liquidity can only become in range by crossing from left to
            // right, when we'll need _more_ token_0 (it's becoming more valuable) so user must provide it
            amount_0 = sqrt_price_math::get_amount_0_delta_signed(
                tick_math::get_sqrt_ratio_at_tick(tick_lower)?,
                tick_math::get_sqrt_ratio_at_tick(tick_upper)?,
                liquidity_delta,
            );
        } else if pool_state.tick < tick_upper {
            // current tick is inside the passed range

            // write oracle observation
            let timestamp = oracle::_block_timestamp();
            let next_observation_start = (latest_observation.block_timestamp / 14 + 1) * 14;
            let mut next_observation = if timestamp >= next_observation_start {
                next_observation_state.load_mut()?
            } else {
                latest_observation
            };
            pool_state.observation_cardinality_next = next_observation.update(
                timestamp,
                pool_state.tick,
                pool_state.liquidity,
                pool_state.observation_cardinality,
                pool_state.observation_cardinality_next,
            );
            pool_state.observation_index = next_observation.index;

            // Both Δtoken_0 and Δtoken_1 will be needed in current price
            amount_0 = sqrt_price_math::get_amount_0_delta_signed(
                pool_state.sqrt_price_x32,
                tick_math::get_sqrt_ratio_at_tick(tick_upper)?,
                liquidity_delta,
            );
            amount_1 = sqrt_price_math::get_amount_1_delta_signed(
                tick_math::get_sqrt_ratio_at_tick(tick_lower)?,
                pool_state.sqrt_price_x32,
                liquidity_delta,
            );

            pool_state.liquidity =
                liquidity_math::add_delta(pool_state.liquidity, liquidity_delta)?;
        }
        // current tick is above the range
        else {
            amount_1 = sqrt_price_math::get_amount_1_delta_signed(
                tick_math::get_sqrt_ratio_at_tick(tick_lower)?,
                tick_math::get_sqrt_ratio_at_tick(tick_upper)?,
                liquidity_delta,
            );
        }
    }

    Ok((amount_0, amount_1))
}

/// Updates a position with the given liquidity delta
///
/// # Arguments
///
/// * `pool_state` - Current pool state
/// * `position_state` - Effect change to this position
/// * `tick_lower_state`- Program account for the lower tick boundary
/// * `tick_upper_state`- Program account for the upper tick boundary
/// * `bitmap_lower` - Bitmap account for the lower tick
/// * `bitmap_upper` - Bitmap account for the upper tick, if it is different from
/// `bitmap_lower`
/// * `lamport_destination` - Destination account for freed lamports when a tick state is
/// un-initialized
/// * `liquidity_delta` - The change in liquidity. Can be 0 to perform a poke.
///
pub fn _update_position<'info>(
    pool_state: &PoolState,
    position_state: &Loader<'info, PositionState>,
    tick_lower_state: &Loader<'info, TickState>,
    tick_upper_state: &Loader<'info, TickState>,
    bitmap_lower: &Loader<'info, TickBitmapState>,
    bitmap_upper: &Loader<'info, TickBitmapState>,
    latest_observation_state: &ObservationState,
    lamport_destination: AccountInfo<'info>,
    liquidity_delta: i64,
) -> ProgramResult {
    let mut tick_lower = tick_lower_state.load_mut()?;
    let mut tick_upper = tick_upper_state.load_mut()?;

    let mut flipped_lower = false;
    let mut flipped_upper = false;

    // update the ticks if liquidity delta is non-zero
    if liquidity_delta != 0 {
        let time = oracle::_block_timestamp();
        let (tick_cumulative, seconds_per_liquidity_cumulative_x32) =
            latest_observation_state.observe_latest(time, pool_state.tick, pool_state.liquidity);

        let max_liquidity_per_tick =
            tick_spacing_to_max_liquidity_per_tick(pool_state.tick_spacing as i32);

        // Update tick state and find if tick is flipped
        flipped_lower = tick_lower.update(
            pool_state.tick,
            liquidity_delta,
            pool_state.fee_growth_global_0_x32,
            pool_state.fee_growth_global_1_x32,
            seconds_per_liquidity_cumulative_x32,
            tick_cumulative,
            time,
            false,
            max_liquidity_per_tick,
        )?;
        flipped_upper = tick_upper.update(
            pool_state.tick,
            liquidity_delta,
            pool_state.fee_growth_global_0_x32,
            pool_state.fee_growth_global_1_x32,
            seconds_per_liquidity_cumulative_x32,
            tick_cumulative,
            time,
            true,
            max_liquidity_per_tick,
        )?;

        if flipped_lower {
            let bit_pos = ((tick_lower.tick / pool_state.tick_spacing as i32) % 256) as u8; // rightmost 8 bits
            bitmap_lower.load_mut()?.flip_tick(bit_pos);
        }
        if flipped_upper {
            let bit_pos = ((tick_upper.tick / pool_state.tick_spacing as i32) % 256) as u8;
            if bitmap_lower.key() == bitmap_upper.key() {
                bitmap_lower.load_mut()?.flip_tick(bit_pos);
            } else {
                bitmap_upper.load_mut()?.flip_tick(bit_pos);
            }
        }
    }
    // Update fees accrued to the position
    let (fee_growth_inside_0_x32, fee_growth_inside_1_x32) = TickState::get_fee_growth_inside(
        tick_upper.deref(),
        tick_lower.deref(),
        pool_state.tick,
        pool_state.fee_growth_global_0_x32,
        pool_state.fee_growth_global_1_x32,
    );
    position_state.load_mut()?.update(
        liquidity_delta,
        fee_growth_inside_0_x32,
        fee_growth_inside_1_x32,
    )?;

    // Deallocate the tick accounts if they get un-initialized
    // A tick is un-initialized on flip if liquidity_delta is negative
    if liquidity_delta < 0 {
        if flipped_lower {
            tick_lower_state.close(lamport_destination.clone())?;
        }
        if flipped_upper {
            tick_lower_state.close(lamport_destination)?;
        }
    }
    Ok(())
}

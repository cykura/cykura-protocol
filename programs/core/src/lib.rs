pub mod context;
pub mod error;
pub mod libraries;
pub mod states;
use crate::libraries::tick_math;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;
use anchor_lang::AccountsClose;
use anchor_spl::{associated_token, token};
use context::*;
use libraries::sqrt_price_math::{get_amount_0_delta_signed, get_amount_1_delta_signed};
use crate::error::ErrorCode;
use states::factory::*;
use states::fee::*;
use states::pool::*;
use states::position::*;
use states::tick::*;
use states::tick_bitmap::*;
use std::convert::TryInto;
use crate::states::oracle::ObservationState;
use std::mem::size_of;
use anchor_lang::solana_program::system_instruction::create_account;
use crate::states::oracle;

declare_id!("37kn8WUzihQoAnhYxueA2BnqCA7VRnrVvYoHy1hQ6Veu");

#[program]
pub mod cyclos_core {
    use std::ops::{Deref, DerefMut};

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
        ctx.accounts.factory_state.bump = factory_state_bump;
        ctx.accounts.factory_state.owner = ctx.accounts.owner.key();

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
        ctx.accounts.factory_state.owner = ctx.accounts.new_owner.key();

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
        assert!(tick_spacing > 0 && tick_spacing < 16384);
        ctx.accounts.fee_state.bump = fee_state_bump;
        ctx.accounts.fee_state.fee = fee;
        ctx.accounts.fee_state.tick_spacing = tick_spacing;

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
        let tick = tick_math::get_tick_at_sqrt_ratio(sqrt_price_x32)?;

        ctx.accounts.pool_state.bump = pool_state_bump;
        ctx.accounts.pool_state.token_0 = ctx.accounts.token_0.key();
        ctx.accounts.pool_state.token_1 = ctx.accounts.token_1.key();
        ctx.accounts.pool_state.fee = ctx.accounts.fee_state.fee;
        ctx.accounts.pool_state.tick_spacing = ctx.accounts.fee_state.tick_spacing;
        ctx.accounts.pool_state.sqrt_price_x32 = sqrt_price_x32;
        ctx.accounts.pool_state.tick = tick;
        ctx.accounts.pool_state.unlocked = true;
        ctx.accounts.pool_state.observation_cardinality = 1;
        ctx.accounts.pool_state.observation_cardinality_next = 1;

        ctx.accounts.initial_observation_state.bump = observation_state_bump;
        ctx.accounts.initial_observation_state.block_timestamp = oracle::_block_timestamp();
        ctx.accounts.initial_observation_state.initialized = true;

        // default value 0 for remaining variables

        emit!(PoolCreatedAndInitialized {
            token_0: ctx.accounts.token_0.key(),
            token_1: ctx.accounts.token_1.key(),
            fee: ctx.accounts.fee_state.fee,
            tick_spacing: ctx.accounts.fee_state.tick_spacing,
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
        observation_account_bumps: Vec<u8>
    ) -> ProgramResult {
        require!(ctx.accounts.pool_state.unlocked, ErrorCode::LOK);
        ctx.accounts.pool_state.unlocked = false;

        let mut i: usize = 0;
        while i < observation_account_bumps.len() {

            let observation_account_seeds = [
                ctx.accounts.pool_state.token_0.as_ref(),
                ctx.accounts.pool_state.token_1.as_ref(),
                &ctx.accounts.pool_state.fee.to_be_bytes(),
                &(ctx.accounts.pool_state.observation_cardinality_next + i as u16).to_be_bytes(),
                &[observation_account_bumps[i]]
            ];

            require!(
                ctx.remaining_accounts[i].key() == Pubkey::create_program_address(
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
                ctx.program_id
            );

            solana_program::program::invoke_signed(
                &ix,
                &[
                    ctx.accounts.payer.to_account_info(),
                    ctx.remaining_accounts[i].to_account_info(),
                    ctx.accounts.system_program.to_account_info()
                ],
                &[&observation_account_seeds[..]]
            )?;

            let mut observation_state = Account::<ObservationState>::try_from_unchecked(
                &ctx.remaining_accounts[i].to_account_info()
            ).unwrap();
            // this data will not be used because the initialized boolean is still false
            observation_state.bump = observation_account_bumps[i];
            observation_state.index = ctx.accounts.pool_state.observation_cardinality_next + i as u16;
            observation_state.block_timestamp = 1;
            observation_state.exit(ctx.program_id)?;

            i += 1;
        }
        let observation_cardinality_next_old = ctx.accounts.pool_state.observation_cardinality_next;
        ctx.accounts.pool_state.observation_cardinality_next += i as u16;

        emit!(oracle::IncreaseObservationCardinalityNext {
            observation_cardinality_next_old,
            observation_cardinality_next_new: ctx.accounts.pool_state.observation_cardinality_next,
        });

        ctx.accounts.pool_state.unlocked = true;
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
        require!(ctx.accounts.pool_state.unlocked, ErrorCode::LOK);
        ctx.accounts.pool_state.unlocked = false;

        assert!(
            (fee_protocol_0 == 0 || (fee_protocol_0 >= 4 && fee_protocol_0 <= 10)) &&
            (fee_protocol_1 == 0 || (fee_protocol_1 >= 4 && fee_protocol_1 <= 10))
        );

        let fee_protocol_old = ctx.accounts.pool_state.fee_protocol;
        ctx.accounts.pool_state.fee_protocol = (fee_protocol_1 << 4) + fee_protocol_0;

        emit!(SetFeeProtocolEvent {
            pool_state: ctx.accounts.pool_state.key(),
            fee_protocol_0_old: fee_protocol_old % 16,
            fee_protocol_1_old: fee_protocol_old >> 4,
            fee_protocol_0,
            fee_protocol_1,
        });

        ctx.accounts.pool_state.unlocked = true;
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
        require!(ctx.accounts.pool_state.unlocked, ErrorCode::LOK);
        ctx.accounts.pool_state.unlocked = false;

        let amount_0 = amount_0_requested.min(ctx.accounts.pool_state.protocol_fees_token_0);
        let amount_1 = amount_1_requested.min(ctx.accounts.pool_state.protocol_fees_token_1);

        let pool_state_seeds = [
            &ctx.accounts.pool_state.token_0.to_bytes() as &[u8],
            &ctx.accounts.pool_state.token_1.to_bytes() as &[u8],
            &ctx.accounts.pool_state.fee.to_be_bytes(),
            &[ctx.accounts.pool_state.bump]
        ];

        if amount_0 > 0 {
            ctx.accounts.pool_state.protocol_fees_token_0 -= amount_0;
            token::transfer(CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_0.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_0.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]]
                ),
                amount_0,
            )?;
        }
        if amount_1 > 0 {
            ctx.accounts.pool_state.protocol_fees_token_1 -= amount_1;
            token::transfer(CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_1.to_account_info().clone(),
                        to: ctx.accounts.recipient_wallet_1.to_account_info().clone(),
                        authority: ctx.accounts.pool_state.to_account_info().clone(),
                    },
                    &[&pool_state_seeds[..]]
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

        ctx.accounts.pool_state.unlocked = true;
        Ok(())
    }

    /// ---------------------------------------------------------------------
    /// Account init instructions
    ///
    /// Having separate instructions to initialize instructions saves compute units
    /// and reduces code in downstream instructions
    ///

    /// Initializes a program account for a price tick
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains accounts to initialize an empty tick account
    /// * `tick_account_bump` - Bump to validate tick account PDA
    /// * `tick` - The tick for which the account is created
    ///
    pub fn init_tick_account(ctx: Context<InitTickAccount>, tick_account_bump: u8, tick: i32) -> ProgramResult {
        let mut tick_account = ctx.accounts.tick_state.load_init()?;
        tick_account.bump = tick_account_bump;
        tick_account.tick = tick;
        Ok(())
    }

    /// Initializes a program account for a tick bitmap
    ///
    /// # Arguments
    ///
    /// * `ctx` - Contains accounts to initialize an empty bitmap account
    /// * `tick_account_bump` - Bump to validate the bitmap account PDA
    /// * `word_position` - The word position for which the account is created, i.e. the i16 word
    /// obtained by right shifting a tick by 8 places
    ///
    pub fn init_bitmap_account(
        ctx: Context<InitBitmapAccount>,
        bitmap_account_bump: u8,
        word_position: i16
    ) -> ProgramResult {
        let mut bitmap_account = ctx.accounts.bitmap_state.load_init()?;
        bitmap_account.bump = bitmap_account_bump;
        bitmap_account.word_position = word_position;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Position instructions

    /// Adds liquidity for the given pool/recipient/tickLower/tickUpper position
    ///
    //tick_abitmap_bump: bitmap_account_bump: bitmap_bumphis method receives a callback in the form of #mintCallback
    /// in which they must pay any token_0 or token_1 owed for the liquidity. The
    /// amounbump: tick_account_bump token_0/token_1 due depends on tickLower, tickUpper, the amount of
    /// liquidity, and the current price.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Holds the recipient's address and program accounts for
    /// pool, position and ticks.
    /// * `tick_lower` - The lower tick of the position in which to add liquidity
    /// * `tick_upper` - The upper tick of the position in which to add liquidity
    /// * `amount` - The amount of liquidity to mint
    ///
    pub fn mint(
        ctx: Context<MintContext>,
        position_bump: u8,
        tick_lower_bump: u8,
        tick_upper_bump: u8,
        bitmap_lower_bump: u8,
        bitmap_upper_bump: u8,
        tick_lower: i32,
        tick_upper: i32,
        amount: u32
    ) -> ProgramResult {
        msg!("inside core#mint");
        // require!(ctx.accounts.pool_state.unlocked, ErrorCode::LOK);
        // msg!("locking");
        // ctx.accounts.pool_state.unlocked = false;

        // ctx.accounts.pool_state.unlocked = true;
        Ok(())
    }

    // pub fn mint(
    //     ctx: Context<MintAccount>,
    //     amount: u32, // Δliquidity
    //     data: [u8; 32]
    // ) -> ProgramResult {
    //     if !ctx.accounts.pool_state.unlocked {
    //         return Err(ErrorCode::Locked.into());
    //     }
    //     ctx.accounts.pool_state.unlocked = false;

    //     // ________________________________________________
    //     require!(amount > 0, ErrorCode::ZeroMintAmount);

    //     // Position, tick and tick_bitmap states may be initialized

    //     // TODO if position_state was initialized, set values
    //     // let pos_is_init = ctx.accounts.position_state.to_account_info().data.into_inner().len() == 8;

    //     // Minter must transfer these amounts to smart contract
    //     // amount_0 and amount_1 will be positive since Δliquidity is positive
    //     let (amount_0, amount_1) = modify_position(
    //         &mut ctx.accounts.position_state,
    //         &mut ctx.accounts.pool_state,
    //         &mut ctx.accounts.tick_lower_state,
    //         &mut ctx.accounts.tick_upper_state,
    //         &mut ctx.accounts.tick_lower_bitmap,
    //         &mut ctx.accounts.tick_upper_bitmap,
    //         amount.try_into().unwrap(),
    //     );

    //     let mut balance_0_before = 0_u64;
    //     let mut balance_1_before = 0_u64;

    //     // Gas optimization: skip comparison if amount was not added
    //     if amount_0 > 0 {
    //         balance_0_before = ctx.accounts.vault_0.amount;
    //     }
    //     if amount_1 > 0 {
    //         balance_1_before = ctx.accounts.vault_1.amount;
    //     }

    //     // Callback to make minter pay
    //     // TODO study encoding format and security
    //     // Uniswap sends amount_0, amount_1, data(bump: factory_state_bumpess interacting with NFT position manager
    //     // and pool key(token_0, token_1, fee))
    //     // We pass borsh serialized message (amount_0_owed, amount_1_owed, arbitrary data) and entire context
    //     // Arbitrary data not needed for Cyclos, but retained for composability

    //     let ix = Instruction::new_with_bytes(
    //         ctx.accounts.minter.key(),
    //         &data,
    //         ctx.accounts.to_account_metas(Some(true))
    //     );

    //     let seeds = &[
    //         &ctx.accounts.pool_state.token_0.to_bytes() as &[u8],
    //         &ctx.accounts.pool_state.token_1.to_bytes() as &[u8],
    //         &ctx.accounts.pool_state.fee.to_be_bytes() as &[u8],
    //     ];
    //     let signer_seeds = &[&seeds[..]];

    // bumps: observation_account_bumps/ let cpi_ctx = CpiContext::from(&*ctx.accounts).with_signer(signer);
    //     // non_fungible_position_manager::cpi::mint_callback(cpi_ctx, data)

    //     // Sign with pool
    //     solana_program::program::invoke_signed(
    //         &ix,
    //         &ctx.accounts.to_account_infos(),
    //         signer_seeds
    //     )?;

    //     // Ensure payment is made. Skip checks if amount was not added
    //     if amount_0 > 0 {
    //         require!(
    //             balance_0_before + (amount_0 as u64) <= ctx.accounts.vault_0.amount,
    //             ErrorCode::M0
    //         );
    //     }
    //     if amount_1 > 0 {
    //         require!(
    //             balance_1_before + (amount_1 as u64) <= ctx.accounts.vault_1.amount,
    //             ErrorCode::M1
    //         );
    //     }

    //     emit!(MintEvent {
    //         pool_state: ctx.accounts.pool_state.key(),
    //         mint_creator: ctx.accounts.minter.key(),
    //         position_state: ctx.accounts.position_state.key(),
    //         tick_lower: ctx.accounts.tick_lower_state.tick,
    //         tick_upper: ctx.accounts.tick_upper_state.tick,
    //         amount,
    //         amount_0,
    //         amount_bump: bumps   //     });

    //     // ______________________________________________
    //     ctx.accounts.pool_state.unlocked = true;
    //     Ok(())
    // }

    // /// Collect tokens owed to a position
    // /// Owed = fees + burned tokens
    // ///
    // /// Does not recompute fees earned, which must be done either via mint or
    // /// burn of any amount of liquidity.
    // /// To withdraw a single asset, the amount for the other asset can be set as 0.
    // /// To withdraw all tokens owed, a value larger than owed amount can be passed,
    // /// e.g. u64::MAX
    // ///
    // pub fn collect(
    //     ctx: Context<MintAccount>,
    //     // TODO Read position details (tick_upper, tick_lower) from the Position PDA
    //     tick_lower: i32,
    //     tick_upper: i32,
    //     amount_0_requested: u64,
    //     amount_1_requested: u64,
    // ) -> ProgramResult {
    //     if !ctx.accounts.pool_state.unlocked {
    //         return Err(ErrorCode::Locked.into());
    //     }
    //     ctx.accounts.pool_state.unlocked = false;
    //     // ______________________________________________

    //     let position_state = &mut *ctx.accounts.position_state;
    //     let pool_state = &mut *ctx.accounts.pool_state;

    //     let amount_0 = if amount_0_requested > position_state.tokens_owed_0 {
    //         position_state.tokens_owed_0
    //     } else {
    //         amount_0_requested
    //     };

    //     let amount_1 = if amount_1_requested > position_state.tokens_owed_1 {
    //         position_state.tokens_owed_1
    //     } else {
    //         amount_1_requested
    //     };

    //     if amount_0 > 0 {
    //         position_state.tokens_owed_0 =
    //             position_state.tokens_owed_0.checked_sub(amount_0).unwrap();
    //         // TODO: Transfer
    //     }
    //     if amount_1 > 0 {
    //         position_state.tokens_owed_1 =
    //             position_state.tokens_owed_1.checked_sub(amount_1).unwrap();
    //         //  TODO: Transfer
    //     }

    //     emit!(CollectEvent {
    //         pool_state: pool_state.key(),
    //         tick_lower,
    //         tick_upper,
    //         amount_0: amount_0 as i64,
    //         amount_1: amount_1 as i64,
    //     });

    //     // ______________________________________________
    //     ctx.accounts.pool_state.unlocked = true;
    //     Ok(())
    // }

    // /// Burn liquidity for the sender and credit to tokens owed for the liquidity to the position
    // /// Poke- Trigger recalculation of fees by calling with amount = 0
    // /// Fees must be collected separately via a call to collect()
    // ///
    // /// # Arguments
    // ///
    // /// * `amount` - Amount of liquidity to be burned
    // ///
    // pub fn burn(
    //     ctx: Context<MintAccount>,
    //     // TODO read tick range from position account
    //     tick_lower: i32,
    //     tick_upper: i32,
    //     amount: u32,
    // ) -> ProgramResult {
    //     if !ctx.accounts.pool_state.unlocked {
    //         return Err(ErrorCode::Locked.into());
    //     }
    //     ctx.accounts.pool_state.unlocked = false;
    //     // ______________________________________________

    //     let position_state = &mut *ctx.accounts.position_state;
    //     let pool_state = &mut *ctx.accounts.pool_state;

    //     // let (amount_0, amount_1) = modify_position(
    //     //     position_state,
    //     //     pool_state,
    //     //     // Need to recheck
    //     //     &mut ctx.remaining_accounts.tick_lower_state,
    //     //     &mut ctx.remaining_accounts.tick_upper_state,
    //     //     0,
    //     //     &mut ctx.remaining_accounts.tick_lower_bitmap,
    //     //     &mut ctx.remaining_accounts.tick_upper_bitmap,
    //     //     0,
    //     // );
    //     // TODO: Make modify_position work

    //     let amount_0 = 0_i64;
    //     let amount_1 = 0_i64;

    //     if amount_0 > 0 || amount_1 > 0 {
    //         position_state.tokens_owed_0 = position_state
    //             .tokens_owed_0
    //             .checked_add(amount_0.abs() as u64)
    //             .unwrap();
    //         position_state.tokens_owed_1 = position_state
    //             .tokens_owed_1
    //             .checked_add(amount_1.abs() as u64)
    //             .unwrap();
    //     }

    //     emit!(BurnEvent {
    //         pool_state: pool_state.key(),
    //         tick_lower,
    //         tick_upper,
    //         amount,
    //         amount_0: amount_0 as i64,
    //         amount_1: amount_1 as i64,
    //     });

    //     // ______________________________________________
    //     ctx.accounts.pool_state.unlocked = true;
    //     Ok(())
    // }

    // // ---------------------------------------------------------------------
    // // 4. Swap instructions

    // /// Perform swap
    // ///
    // /// Only callable by smart contract which implements uniswapV3SwapCallback()
    // ///
    // /// Flow
    // /// 1. Periphery.SwapRouter.exactInputInternal()/exactOutputInternal(): stateless routing
    // /// 2. Core.UniswapV3Pool.swap(): change state
    // /// 3. Periphery.SwapRouter.uniswapV3SwapCallback(): transfer tokens from user to pool
    // ///
    // /// @param zero_for_one Swap token0 -> token1 if true, else token1 -> token0
    // /// @param amount_specified Δtoken0 or Δtoken1 to be added/removed to pool.
    // /// Exact input swap if positive, else exact output swap
    // /// @param sqrt_price_limit Limit price √P for slippage
    // pub fn swap(
    //     ctx: Context<SetFeeProtocol>,
    //     zero_for_one: bool,
    //     amount_specified: i64,
    //     sqrt_price_limit: f64,
    // ) -> ProgramResult {
    //     todo!()
    // }

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

// /// Update position with given liquidity_delta
// /// Skipped TWAP calculation for now.
// /// Position liquidity and flipped state in bitmap is updated
// /// From Pools._update_position()
// pub fn update_position<'info>(
//     position_state: &mut Account<'info, PositionState>,
//     pool_state: &Account<'info, PoolState>,
//     tick_lower_state: &mut Account<'info, TickState>,
//     tick_upper_state: &mut Account<'info, TickState>,
//     tick_lower_bitmap: &mut Account<'info, TickBitmapState>, // must contain tick
//     tick_upper_bitmap: &mut Account<'info, TickBitmapState>,
//     liquidity_delta: i32,
// ) -> ProgramResult {
//     let mut flipped_lower = false;
//     let mut flipped_upper = false;

//     // update the ticks if liquidity present
//     if liquidity_delta != 0 {
//         let max_liquidity_per_tick =
//             tick_spacing_to_max_liquidity_per_tick(pool_state.tick_spacing as i32);
//         // Skip TWAP things for now.

//         // Update tick state and find if tick is flipped
//         flipped_lower = tick_lower_state.update(
//             pool_state.tick,
//             liquidity_delta,
//             pool_state.fee_growth_global_0,
//             pool_state.fee_growth_global_1,
//             false,
//             max_liquidity_per_tick,
//         );

//         flipped_upper = tick_upper_state.update(
//             pool_state.tick,
//             liquidity_delta,
//             pool_state.fee_growth_global_0,
//             pool_state.fee_growth_global_1,
//             true,
//             max_liquidity_per_tick,
//         );

//         if flipped_lower {
//             let (_, bit_pos) =
//                 get_word_and_bit_pos(tick_lower_state.tick / (pool_state.tick_spacing as i32));
//             tick_lower_bitmap.flip_tick(bit_pos);
//         }
//         if flipped_upper {
//             let (_, bit_pos) =
//                 get_word_and_bit_pos(tick_upper_state.tick / (pool_state.tick_spacing as i32));
//             tick_upper_bitmap.flip_tick(bit_pos);
//         }
//     }
//     // Update fees for position
//     // Poke: to only update fees, liquidity_delta can be passed as 0

//     let (fee_growth_inside_0, fee_growth_inside_1) = TickState::get_fee_growth_inside(
//         tick_upper_state,
//         tick_lower_state,
//         pool_state.tick,
//         pool_state.fee_growth_global_0,
//         pool_state.fee_growth_global_1,
//     );
//     position_state.update(liquidity_delta, fee_growth_inside_0, fee_growth_inside_1);

//     // Deallocate a tick if it gets uninitialized
//     // If tick is flipped and liquidity_delta is negative, it gets uninitialized
//     if liquidity_delta < 0 {
//         if flipped_lower {
//             tick_lower_state.clear();
//             return tick_lower_state.close(position_state.to_account_info());
//         }
//         if flipped_upper {
//             tick_upper_state.clear();
//             return tick_upper_state.close(position_state.to_account_info());
//         }
//     }
//     Ok(())
// }

// /// Update position with new liquidity, and find Δtoken0 and Δtoken1 required
// /// to produce this liquidity_delta
// /// mint() -> modify_position() -> update_position() -> update()
// /// Poking: Δliquidity = 0 to update fee status of a position
// ///
// /// Return Δtoken0 and Δtoken1 required to produce the given change in liquidity
// /// TODO check what noDelegateCall does
// pub fn modify_position<'info>(
//     position_state: &mut Account<'info, PositionState>,
//     pool_state: &mut Account<'info, PoolState>,
//     tick_lower_state: &mut Account<'info, TickState>,
//     tick_upper_state: &mut Account<'info, TickState>,
//     // These are only used in update position.
//     // TODO: Need to check the states being passed for redundancy
//     tick_lower_bitmap: &mut Account<'info, TickBitmapState>, // must contain tick
//     tick_upper_bitmap: &mut Account<'info, TickBitmapState>,
//     liquidity_delta: i32,
// ) -> (i64, i64) {
//     // check ticks are in range
//     PoolState::check_ticks(position_state.tick_lower, position_state.tick_upper);

//     let _position = update_position(
//         position_state,
//         pool_state,
//         tick_lower_state,
//         tick_upper_state,
//         tick_lower_bitmap,
//         tick_upper_bitmap,
//         liquidity_delta,
//     ).unwrap();

//     let mut amount_0 = 0_i64;
//     let mut amount_1 = 0_i64;

//     if liquidity_delta != 0 {
//         // current tick is below the range
//         if pool_state.tick < tick_lower_state.tick {
//             amount_0 = get_amount_0_delta_signed(
//                 get_sqrt_price_at_tick(tick_lower_state.tick),
//                 get_sqrt_price_at_tick(tick_upper_state.tick),
//                 liquidity_delta,
//             );
//             // Δtoken_1 will be 0 if current tick is below range
//         }
//         // current tick is within the range
//         else if pool_state.tick < tick_upper_state.tick {
//             // skipped oracle entry for now

//             // Both Δtoken_0 and Δtoken_1 will be needed in current price
//             amount_0 = get_amount_0_delta_signed(
//                 pool_state.sqrt_price,
//                 get_sqrt_price_at_tick(tick_upper_state.tick),
//                 liquidity_delta,
//             );
//             amount_1 = get_amount_1_delta_signed(
//                 get_sqrt_price_at_tick(tick_lower_state.tick),
//                 pool_state.sqrt_price,
//                 liquidity_delta,
//             );

//             // Add or subtract Δliquidity to pool state
//             pool_state.liquidity = if liquidity_delta.is_positive() {
//                 pool_state
//                     .liquidity
//                     .checked_add(liquidity_delta.abs() as u32)
//                     .unwrap()
//             } else {
//                 pool_state
//                     .liquidity
//                     .checked_sub(liquidity_delta.abs() as u32)
//                     .unwrap()
//             };
//         }
//         // current tick is above the range
//         else {
//             amount_1 = get_amount_1_delta_signed(
//                 get_sqrt_price_at_tick(tick_lower_state.tick),
//                 get_sqrt_price_at_tick(tick_upper_state.tick),
//                 liquidity_delta,
//             );
//             // Δtoken_0 will be 0 if current tick is below range
//         }
//     }

//     (amount_0, amount_1)
// }

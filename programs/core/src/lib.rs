pub mod context;
pub mod error;
pub mod libraries;
pub mod states;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::AccountsClose;
use anchor_spl::{associated_token, token};
use context::*;
use error::ErrorCode;
use libraries::sqrt_price_math::{get_amount_0_delta_signed, get_amount_1_delta_signed};
use libraries::tick_math::{get_sqrt_price_at_tick, get_tick_at_sqrt_price};
use states::factory::*;
use states::fee::*;
use states::pool::*;
use states::position::*;
use states::tick::*;
use states::tick_bitmap::*;
use std::mem::size_of;

declare_id!("37kn8WUzihQoAnhYxueA2BnqCA7VRnrVvYoHy1hQ6Veu");

#[program]
pub mod cyclos_core {
    use std::convert::TryInto;

    use anchor_lang::solana_program::{self, instruction::Instruction};

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

        emit!(FeeAmountEnabledEvent { fee, tick_spacing });

        ctx.accounts.fee_state.bump = fee_bump;
        ctx.accounts.fee_state.fee = fee;
        ctx.accounts.fee_state.tick_spacing = tick_spacing;

        Ok(())
    }

    pub fn set_owner(ctx: Context<SetOwner>) -> ProgramResult {
        ctx.accounts.factory_state.owner = ctx.accounts.new_owner.key();

        emit!(OwnerChangedEvent {
            old_owner: ctx.accounts.owner.key(),
            new_owner: ctx.accounts.new_owner.key(),
        });

        Ok(())
    }

    // ---------------------------------------------------------------------
    // 2. Pool instructions

    /// Create pool and initialize with desired price
    /// Create pool PDA for [token0, token1, fee] where tokenA > tokenB,
    /// then set sqrt_price
    ///
    /// Single function in place of Factory.createPool(), PoolDeployer.deploy()
    /// Pool.initialize() and pool.Constructor()
    pub fn create_pool(
        ctx: Context<CreatePool>,
        pool_state_bump: u8,
        fee: u32,
        sqrt_price: f64,
    ) -> ProgramResult {
        // let state = PoolState {
        //     bump: pool_state_bump,
        //     token_0: (*ctx.accounts.token_0).key(),
        //     token_1: (*ctx.accounts.token_1).key(),
        //     fee,
        //     tick_spacing: (*ctx.accounts.fee_state).tick_spacing,
        //     liquidity: 0,
        //     sqrt_price,
        //     tick: get_tick_at_sqrt_price(sqrt_price),
        //     fee_growth_global_0: 0.0,
        //     fee_growth_global_1: 0.0,
        //     fee_protocol: 0, // Leftmost 4 bits: fee_token_0, rightmost 4 bits: fee_token_1
        //     protocol_fees_token_0: 0,
        //     protocol_fees_token_1: 0,
        //     unlocked: true,
        // };

        // ctx.accounts.pool_state.clone()
        // *ctx.accounts.pool_state = state as Account;

        let tick = get_tick_at_sqrt_price(sqrt_price);

        // Set pool state
        ctx.accounts.pool_state.bump = pool_state_bump;
        ctx.accounts.pool_state.token_0 = (*ctx.accounts.token_0).key();
        ctx.accounts.pool_state.token_1 = (*ctx.accounts.token_1).key();
        ctx.accounts.pool_state.fee = fee;
        ctx.accounts.pool_state.tick_spacing = (*ctx.accounts.fee_state).tick_spacing;
        ctx.accounts.pool_state.sqrt_price = sqrt_price;
        ctx.accounts.pool_state.tick = tick;
        ctx.accounts.pool_state.unlocked = true;
        // protocol fee initially set as 0

        // create pool vaults if not created before
        // These are ATAs for the pool token pair, with pool_state PDA as authority
        if !ctx.accounts.vault_0.executable {
            let create_vault_0_ctx = CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                associated_token::Create {
                    payer: ctx.accounts.pool_creator.to_account_info(),
                    associated_token: ctx.accounts.vault_0.to_account_info(),
                    authority: ctx.accounts.pool_state.to_account_info(),
                    mint: ctx.accounts.token_0.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            );
            associated_token::create(create_vault_0_ctx)?;
        }
        if !ctx.accounts.vault_1.executable {
            let create_vault_1_ctx = CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                associated_token::Create {
                    payer: ctx.accounts.pool_creator.to_account_info(),
                    associated_token: ctx.accounts.vault_1.to_account_info(),
                    authority: ctx.accounts.pool_state.to_account_info(),
                    mint: ctx.accounts.token_1.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                },
            );
            associated_token::create(create_vault_1_ctx)?;
        }

        emit!(InitPoolEvent {
            pool_state: (*ctx.accounts.pool_state).key(),
            token_0: (*ctx.accounts.token_0).key(),
            token_1: (*ctx.accounts.token_1).key(),
            fee,
            sqrt_price,
            tick,
        });

        Ok(())
    }

    // ---------------------------------------------------------------------
    // 3. Position instructions

    /// Create a new position or add liquidity to an existing one
    ///
    /// Caller must be a smart contract implementing mint_callback(), where due
    /// tokens are paid
    /// mint() / increaseLiquidity() -> Periphery.LiquidityManagement.addLiquidity()
    /// -> Core.mint() -> Periphery.LiquidityManagement.uniswapV3MintCallback()
    ///
    /// Uniswap has a data field which is passed to uniswapV3MintCallback().
    /// This contains pool key(t0, t1, fee) and msg.sender. It is required by the
    /// perephery NFT position manager. 3rd parties may need more params for flexibility
    /// We can implement this through a data: &[u8] field and remaining_accounts in context
    pub fn mint(
        ctx: Context<MintAccount>,
        amount: u32, // Δliquidity
        data: [u8; 32]
    ) -> ProgramResult {
        if !ctx.accounts.pool_state.unlocked {
            return Err(ErrorCode::Locked.into());
        }
        ctx.accounts.pool_state.unlocked = false;

        // ________________________________________________
        require!(amount > 0, ErrorCode::ZeroMintAmount);

        // Position, tick and tick_bitmap states may be initialized

        // TODO if position_state was initialized, set values
        // let pos_is_init = ctx.accounts.position_state.to_account_info().data.into_inner().len() == 8;


        // Minter must transfer these amounts to smart contract
        // amount_0 and amount_1 will be positive since Δliquidity is positive
        let (amount_0, amount_1) = modify_position(
            &mut ctx.accounts.position_state,
            &mut ctx.accounts.pool_state,
            &mut ctx.accounts.tick_lower_state,
            &mut ctx.accounts.tick_upper_state,
            &mut ctx.accounts.tick_lower_bitmap,
            &mut ctx.accounts.tick_upper_bitmap,
            amount.try_into().unwrap(),
        );

        let mut balance_0_before = 0_u64;
        let mut balance_1_before = 0_u64;

        // Gas optimization: skip comparison if amount was not added
        if amount_0 > 0 {
            balance_0_before = ctx.accounts.vault_0.amount;
        }
        if amount_1 > 0 {
            balance_1_before = ctx.accounts.vault_1.amount;
        }

        // Callback to make minter pay
        // TODO study encoding format and security
        // Uniswap sends amount_0, amount_1, data(address interacting with NFT position manager
        // and pool key(token_0, token_1, fee))
        // We pass borsh serialized message (amount_0_owed, amount_1_owed, arbitrary data) and entire context
        // Arbitrary data not needed for Cyclos, but retained for composability

        let ix = Instruction::new_with_bytes(
            ctx.accounts.minter.key(),
            &data,
            ctx.accounts.to_account_metas(Some(true))
        );

        let seeds = &[
            &ctx.accounts.pool_state.token_0.to_bytes() as &[u8],
            &ctx.accounts.pool_state.token_1.to_bytes() as &[u8],
            &ctx.accounts.pool_state.fee.to_be_bytes() as &[u8],
        ];
        let signer_seeds = &[&seeds[..]];


        // let cpi_ctx = CpiContext::from(&*ctx.accounts).with_signer(signer);
        // non_fungible_position_manager::cpi::mint_callback(cpi_ctx, data)

        // Sign with pool
        solana_program::program::invoke_signed(
            &ix,
            &ctx.accounts.to_account_infos(),
            signer_seeds
        )?;

        // Ensure payment is made. Skip checks if amount was not added
        if amount_0 > 0 {
            require!(
                balance_0_before + (amount_0 as u64) <= ctx.accounts.vault_0.amount,
                ErrorCode::M0
            );
        }
        if amount_1 > 0 {
            require!(
                balance_1_before + (amount_1 as u64) <= ctx.accounts.vault_1.amount,
                ErrorCode::M1
            );
        }

        emit!(MintEvent {
            pool_state: ctx.accounts.pool_state.key(),
            mint_creator: ctx.accounts.minter.key(),
            position_state: ctx.accounts.position_state.key(),
            tick_lower: ctx.accounts.tick_lower_state.tick,
            tick_upper: ctx.accounts.tick_upper_state.tick,
            amount,
            amount_0,
            amount_1,
        });

        // ______________________________________________
        ctx.accounts.pool_state.unlocked = true;
        Ok(())
    }

    /// Collect tokens owed to a position
    /// Owed = fees + burned tokens
    ///
    /// Does not recompute fees earned, which must be done either via mint or
    /// burn of any amount of liquidity.
    /// To withdraw a single asset, the amount for the other asset can be set as 0.
    /// To withdraw all tokens owed, a value larger than owed amount can be passed,
    /// e.g. u64::MAX
    ///
    pub fn collect(
        ctx: Context<MintAccount>,
        // TODO Read position details (tick_upper, tick_lower) from the Position PDA
        tick_lower: i32,
        tick_upper: i32,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> ProgramResult {
        if !ctx.accounts.pool_state.unlocked {
            return Err(ErrorCode::Locked.into());
        }
        ctx.accounts.pool_state.unlocked = false;
        // ______________________________________________

        let position_state = &mut *ctx.accounts.position_state;
        let pool_state = &mut *ctx.accounts.pool_state;

        let amount_0 = if amount_0_requested > position_state.tokens_owed_0 {
            position_state.tokens_owed_0
        } else {
            amount_0_requested
        };

        let amount_1 = if amount_1_requested > position_state.tokens_owed_1 {
            position_state.tokens_owed_1
        } else {
            amount_1_requested
        };

        if amount_0 > 0 {
            position_state.tokens_owed_0 =
                position_state.tokens_owed_0.checked_sub(amount_0).unwrap();
            // TODO: Transfer
        }
        if amount_1 > 0 {
            position_state.tokens_owed_1 =
                position_state.tokens_owed_1.checked_sub(amount_1).unwrap();
            //  TODO: Transfer
        }

        emit!(CollectEvent {
            pool_state: pool_state.key(),
            tick_lower,
            tick_upper,
            amount_0: amount_0 as i64,
            amount_1: amount_1 as i64,
        });

        // ______________________________________________
        ctx.accounts.pool_state.unlocked = true;
        Ok(())
    }

    /// Burn liquidity for the sender and credit to tokens owed for the liquidity to the position
    /// Poke- Trigger recalculation of fees by calling with amount = 0
    /// Fees must be collected separately via a call to collect()
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of liquidity to be burned
    ///
    pub fn burn(
        ctx: Context<MintAccount>,
        // TODO read tick range from position account
        tick_lower: i32,
        tick_upper: i32,
        amount: u32,
    ) -> ProgramResult {
        if !ctx.accounts.pool_state.unlocked {
            return Err(ErrorCode::Locked.into());
        }
        ctx.accounts.pool_state.unlocked = false;
        // ______________________________________________

        let position_state = &mut *ctx.accounts.position_state;
        let pool_state = &mut *ctx.accounts.pool_state;

        // let (amount_0, amount_1) = modify_position(
        //     position_state,
        //     pool_state,
        //     // Need to recheck
        //     &mut ctx.remaining_accounts.tick_lower_state,
        //     &mut ctx.remaining_accounts.tick_upper_state,
        //     0,
        //     &mut ctx.remaining_accounts.tick_lower_bitmap,
        //     &mut ctx.remaining_accounts.tick_upper_bitmap,
        //     0,
        // );
        // TODO: Make modify_position work

        let amount_0 = 0_i64;
        let amount_1 = 0_i64;

        if amount_0 > 0 || amount_1 > 0 {
            position_state.tokens_owed_0 = position_state
                .tokens_owed_0
                .checked_add(amount_0.abs() as u64)
                .unwrap();
            position_state.tokens_owed_1 = position_state
                .tokens_owed_1
                .checked_add(amount_1.abs() as u64)
                .unwrap();
        }

        emit!(BurnEvent {
            pool_state: pool_state.key(),
            tick_lower,
            tick_upper,
            amount,
            amount_0: amount_0 as i64,
            amount_1: amount_1 as i64,
        });

        // ______________________________________________
        ctx.accounts.pool_state.unlocked = true;
        Ok(())
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
        ctx: Context<SetFeeProtocol>,
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
    pub fn flash(ctx: Context<SetFeeProtocol>, amount_0: u64, amount_1: u64) -> ProgramResult {
        todo!()
    }

    // ---------------------------------------------------------------------
    // 5. Pool owner instructions

    /// Update protocol fees for a pool
    /// Protocol fee can be 0 or 1/N where 4 <= N <= 10 (fits in 4 bits)
    /// Both tokens in the pool can have different protocol fees
    /// Compress as a single u8, where fee_protocol_1 are leftmost bits and fee_protocol_0 are rightmost
    pub fn set_fee_protocol(
        ctx: Context<SetFeeProtocol>,
        fee_protocol_0: u8,
        fee_protocol_1: u8,
    ) -> ProgramResult {
        if !ctx.accounts.pool_state.unlocked {
            return Err(ErrorCode::Locked.into());
        }
        ctx.accounts.pool_state.unlocked = false;

        if (fee_protocol_0 == 0 || (fee_protocol_0 >= 4 && fee_protocol_0 <= 10))
            && (fee_protocol_1 == 0 || (fee_protocol_1 >= 4 && fee_protocol_1 <= 10))
        {
            msg!("Error: Protocol fee should be 0 or 1/N where 4 <= N <= 10 ")
        }

        let fee_protocol_old = ctx.accounts.pool_state.fee_protocol;
        // 8 bits = [4 bits of fee_protocol_1][4 bits of fee_protocol_0]
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

    /// Collect protocol fees
    /// Amounts can be 0 to collect fees only in the other token
    pub fn collect_protocol(
        ctx: Context<CollectProtocol>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> ProgramResult {
        if !ctx.accounts.pool_state.unlocked {
            return Err(ErrorCode::Locked.into());
        }
        ctx.accounts.pool_state.unlocked = false;
        let pool_state = &mut *ctx.accounts.pool_state;

        // Amounts to be transferred to owner = MIN (requested, accrued)
        // Cannot transfer out more than accrued
        let mut amount_0 = if amount_0_requested > pool_state.protocol_fees_token_0 {
            pool_state.protocol_fees_token_0
        } else {
            amount_0_requested
        };
        let mut amount_1 = if amount_1_requested > pool_state.protocol_fees_token_1 {
            pool_state.protocol_fees_token_1
        } else {
            amount_1_requested
        };

        let token_0 = pool_state.token_0.clone();
        let token_1 = pool_state.token_1.clone();

        let seeds = &[
            &token_0.to_bytes() as &[u8],
            &token_1.to_bytes() as &[u8],
            &pool_state.fee.to_be_bytes() as &[u8],
            &[pool_state.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        if amount_0 > 0 {
            // Note- Uniswap leaves out 1 fee unit in state so
            // register is not cleared. This saves gas.
            // If there are 100 unclaimed units, maxiumum 99 can be sent
            // Retained for API compatibility
            if amount_0 == pool_state.protocol_fees_token_0 {
                amount_0 = amount_0.checked_sub(1).unwrap();
            }

            pool_state.protocol_fees_token_0 = pool_state
                .protocol_fees_token_0
                .checked_sub(amount_0)
                .unwrap();

            // Transfer
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_0.to_account_info().clone(),
                        to: ctx.accounts.owner_wallet_0.to_account_info().clone(),
                        authority: pool_state.to_account_info().clone(),
                    },
                    signer_seeds,
                ),
                amount_0,
            )?; //handle error. Possible for it to remain locked , make unlocked=true
        }
        if amount_1 > 0 {
            // Note- Uniswap leaves out 1 fee unit in state so
            // register is not cleared. This saves gas.
            // If there are 100 unclaimed units, maxiumum 99 can be sent
            if amount_1 == pool_state.protocol_fees_token_1 {
                amount_1 = amount_1.checked_sub(1).unwrap();
            }

            pool_state.protocol_fees_token_1 = pool_state
                .protocol_fees_token_1
                .checked_sub(amount_1)
                .unwrap();

            // Transfer
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.vault_1.to_account_info().clone(),
                        to: ctx.accounts.owner_wallet_1.to_account_info().clone(),
                        authority: pool_state.to_account_info().clone(),
                    },
                    signer_seeds,
                ),
                amount_1,
            )?; //handle error , make unlocked=true
        }

        emit!(CollectProtocolEvent {
            pool_state: pool_state.key(),
            amount_0,
            amount_1,
        });

        ctx.accounts.pool_state.unlocked = true;
        Ok(())
    }
}

/// Update position with given liquidity_delta
/// Skipped TWAP calculation for now.
/// Position liquidity and flipped state in bitmap is updated
/// From Pools._update_position()
pub fn update_position<'info>(
    position_state: &mut Account<'info, PositionState>,
    pool_state: &Account<'info, PoolState>,
    tick_lower_state: &mut Account<'info, TickState>,
    tick_upper_state: &mut Account<'info, TickState>,
    tick_lower_bitmap: &mut Account<'info, TickBitmapState>, // must contain tick
    tick_upper_bitmap: &mut Account<'info, TickBitmapState>,
    liquidity_delta: i32,
) -> ProgramResult {
    let mut flipped_lower = false;
    let mut flipped_upper = false;

    // update the ticks if liquidity present
    if liquidity_delta != 0 {
        let max_liquidity_per_tick =
            tick_spacing_to_max_liquidity_per_tick(pool_state.tick_spacing as i32);
        // Skip TWAP things for now.

        // Update tick state and find if tick is flipped
        flipped_lower = tick_lower_state.update(
            pool_state.tick,
            liquidity_delta,
            pool_state.fee_growth_global_0,
            pool_state.fee_growth_global_1,
            false,
            max_liquidity_per_tick,
        );

        flipped_upper = tick_upper_state.update(
            pool_state.tick,
            liquidity_delta,
            pool_state.fee_growth_global_0,
            pool_state.fee_growth_global_1,
            true,
            max_liquidity_per_tick,
        );

        if flipped_lower {
            let (_, bit_pos) =
                get_word_and_bit_pos(tick_lower_state.tick / (pool_state.tick_spacing as i32));
            tick_lower_bitmap.flip_tick(bit_pos);
        }
        if flipped_upper {
            let (_, bit_pos) =
                get_word_and_bit_pos(tick_upper_state.tick / (pool_state.tick_spacing as i32));
            tick_upper_bitmap.flip_tick(bit_pos);
        }
    }
    // Update fees for position
    // Poke: to only update fees, liquidity_delta can be passed as 0

    let (fee_growth_inside_0, fee_growth_inside_1) = TickState::get_fee_growth_inside(
        tick_upper_state,
        tick_lower_state,
        pool_state.tick,
        pool_state.fee_growth_global_0,
        pool_state.fee_growth_global_1,
    );
    position_state.update(liquidity_delta, fee_growth_inside_0, fee_growth_inside_1);

    // Deallocate a tick if it gets uninitialized
    // If tick is flipped and liquidity_delta is negative, it gets uninitialized
    if liquidity_delta < 0 {
        if flipped_lower {
            tick_lower_state.clear();
            return tick_lower_state.close(position_state.to_account_info());
        }
        if flipped_upper {
            tick_upper_state.clear();
            return tick_upper_state.close(position_state.to_account_info());
        }
    }
    Ok(())
}

/// Update position with new liquidity, and find Δtoken0 and Δtoken1 required
/// to produce this liquidity_delta
/// mint() -> modify_position() -> update_position() -> update()
/// Poking: Δliquidity = 0 to update fee status of a position
///
/// Return Δtoken0 and Δtoken1 required to produce the given change in liquidity
/// TODO check what noDelegateCall does
pub fn modify_position<'info>(
    position_state: &mut Account<'info, PositionState>,
    pool_state: &mut Account<'info, PoolState>,
    tick_lower_state: &mut Account<'info, TickState>,
    tick_upper_state: &mut Account<'info, TickState>,
    // These are only used in update position.
    // TODO: Need to check the states being passed for redundancy
    tick_lower_bitmap: &mut Account<'info, TickBitmapState>, // must contain tick
    tick_upper_bitmap: &mut Account<'info, TickBitmapState>,
    liquidity_delta: i32,
) -> (i64, i64) {
    // check ticks are in range
    PoolState::check_ticks(position_state.tick_lower, position_state.tick_upper);

    let _position = update_position(
        position_state,
        pool_state,
        tick_lower_state,
        tick_upper_state,
        tick_lower_bitmap,
        tick_upper_bitmap,
        liquidity_delta,
    ).unwrap();

    let mut amount_0 = 0_i64;
    let mut amount_1 = 0_i64;

    if liquidity_delta != 0 {
        // current tick is below the range
        if pool_state.tick < tick_lower_state.tick {
            amount_0 = get_amount_0_delta_signed(
                get_sqrt_price_at_tick(tick_lower_state.tick),
                get_sqrt_price_at_tick(tick_upper_state.tick),
                liquidity_delta,
            );
            // Δtoken_1 will be 0 if current tick is below range
        }
        // current tick is within the range
        else if pool_state.tick < tick_upper_state.tick {
            // skipped oracle entry for now

            // Both Δtoken_0 and Δtoken_1 will be needed in current price
            amount_0 = get_amount_0_delta_signed(
                pool_state.sqrt_price,
                get_sqrt_price_at_tick(tick_upper_state.tick),
                liquidity_delta,
            );
            amount_1 = get_amount_1_delta_signed(
                get_sqrt_price_at_tick(tick_lower_state.tick),
                pool_state.sqrt_price,
                liquidity_delta,
            );

            // Add or subtract Δliquidity to pool state
            pool_state.liquidity = if liquidity_delta.is_positive() {
                pool_state
                    .liquidity
                    .checked_add(liquidity_delta.abs() as u32)
                    .unwrap()
            } else {
                pool_state
                    .liquidity
                    .checked_sub(liquidity_delta.abs() as u32)
                    .unwrap()
            };
        }
        // current tick is above the range
        else {
            amount_1 = get_amount_1_delta_signed(
                get_sqrt_price_at_tick(tick_lower_state.tick),
                get_sqrt_price_at_tick(tick_upper_state.tick),
                liquidity_delta,
            );
            // Δtoken_0 will be 0 if current tick is below range
        }
    }

    (amount_0, amount_1)
}

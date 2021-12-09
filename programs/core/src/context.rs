use anchor_lang::prelude::*;
use anchor_spl::associated_token::{get_associated_token_address, AssociatedToken};
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::mem::size_of;
use std::thread::AccessError;
// TODO remove size_of for initializing PDAs. Use Default attribute instead

use crate::error::ErrorCode;
use crate::states::factory::FactoryState;
use crate::states::fee::FeeState;
use crate::states::pool::PoolState;
use crate::states::position::{POSITION_SEED, PositionState};
use crate::states::oracle::ObservationState;
use crate::states::tick::TickState;
use crate::states::tick_bitmap::{BITMAP_SEED, TickBitmapState};

// use non_fungible_position_manager::program::NonFungiblePositionManager;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Initialize<'info> {
    /// Address to be set as protocol owner. It pays to create factory state account.
    pub owner: Signer<'info>,

    /// Initialize factory state account to store protocol owner address
    #[account(
        init,
        seeds = [],
        bump = bump,
        payer = owner,
        space = 8 + size_of::<FactoryState>()
    )]
    pub factory_state: Box<Account<'info, FactoryState>>,

    /// To create a new program account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee_state_bump: u8, fee: u32, tick_spacing: u16)]
pub struct EnableFeeAmount<'info> {
    /// Valid protocol owner
    #[account(address = factory_state.owner)]
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Box<Account<'info, FactoryState>>,

    /// Initialize an account to store new fee tier and tick spacing
    /// Fees are paid by owner
    #[account(
        init,
        seeds = [&fee.to_be_bytes()],
        bump = fee_state_bump,
        payer = owner,
        space = 8 + size_of::<FeeState>()
    )]
    pub fee_state: Box<Account<'info, FeeState>>,

    /// To create a new program account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOwner<'info> {
    /// Current protocol owner
    #[account(address = factory_state.owner)]
    pub owner: Signer<'info>,

    /// Address to be designated as new protocol owner
    pub new_owner: AccountInfo<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Box<Account<'info, FactoryState>>,
}

#[derive(Accounts)]
#[instruction(pool_state_bump: u8, observation_state_bump: u8)]
pub struct CreateAndInitPool<'info> {
    /// Address paying to create the pool. Can be anyone
    pub pool_creator: Signer<'info>,

    /// Desired token pair for the pool
    /// token_0 mint address should be smaller than token_1 address
    #[account(
        constraint = token_0.key() < token_1.key()
    )]
    pub token_0: Box<Account<'info, Mint>>,
    pub token_1: Box<Account<'info, Mint>>,

    /// Stores the desired fee for the pool
    pub fee_state: Box<Account<'info, FeeState>>,

    /// Initialize an account to store the pool state
    #[account(
        init,
        seeds = [
            token_0.key().as_ref(),
            token_1.key().as_ref(),
            &fee_state.fee.to_be_bytes()
        ],
        bump = pool_state_bump,
        payer = pool_creator,
    )]
    pub pool_state: Loader<'info, PoolState>,

    /// Initialize an account to store oracle observations
    #[account(
        init,
        seeds = [
            token_0.key().as_ref(),
            token_1.key().as_ref(),
            &fee_state.fee.to_be_bytes(),
            &0_u16.to_be_bytes(),
        ],
        bump = observation_state_bump,
        payer = pool_creator,
        space = 8 + size_of::<ObservationState>()
    )]
    pub initial_observation_state: Box<Account<'info, ObservationState>>,

    /// The address that holds pool tokens for token_0
    #[account(
        init_if_needed,
        associated_token::mint = token_0,
        associated_token::authority = pool_state,
        payer = pool_creator,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        init_if_needed,
        associated_token::mint = token_1,
        associated_token::authority = pool_state,
        payer = pool_creator,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// To create a new program account
    pub system_program: Program<'info, System>,

    /// Sysvar for program account and ATA creation
    pub rent: Sysvar<'info, Rent>,

    /// To create new token accounts for the pool
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct IncreaseObservationCardinalityNext<'info> {
    /// Pays to increase storage slots for oracle observations
    pub payer: Signer<'info>,

    /// Increase observation slots for this pool
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,

    /// To create new program accounts
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetFeeProtocol<'info> {
    /// Valid protocol owner
    #[account(address = factory_state.owner)]
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    pub factory_state: Box<Account<'info, FactoryState>>,

    /// Set protocol fee for this pool
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,
}

#[derive(Accounts)]
pub struct CollectProtocol<'info> {
    /// Valid protocol owner
    pub owner: Signer<'info>,

    /// Factory state stores the protocol owner address
    #[account(mut)]
    pub factory_state: Box<Account<'info, FactoryState>>,

    /// Pool state stores accumulated protocol fee amount
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,

    /// The address that holds pool tokens for token_0
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_0.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_1.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The address that receives the collected token_0 protocol fees
    #[account(mut)]
    pub recipient_wallet_0: Box<Account<'info, TokenAccount>>,

    /// The address that receives the collected token_1 protocol fees
    #[account(mut)]
    pub recipient_wallet_1: Box<Account<'info, TokenAccount>>,

    /// The SPL program to perform token transfers
    pub token_program: Program<'info, Token>,
}

// #[derive(Accounts)]
// #[instruction(fee: u32, token_0: Pubkey, token_1: Pubkey, tick_lower:u128, tick_upper:u128, bump: u8)]
// pub struct CreatePosition<'info> {
//     pub owner: Signer<'info>,
//     #[account(
//         init, // should be mut ?
//         seeds = [token_0.as_ref(), token_1.as_ref(), &fee.to_be_bytes(), &tick_lower.to_be_bytes(), &tick_upper.to_be_bytes()],
//         bump = bump,
//         payer = owner,
//         space = size_of::<FeeState>() + 10
//     )]
//     pub pool_state: Box<Account<'info, PositionState>>,
//     pub system_program: Program<'info, System>,
// }

#[derive(Accounts)]
#[instruction(tick_account_bump: u8, tick: i32)]
pub struct InitTickAccount<'info> {
    /// Pays to create tick account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Create a tick account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The tick account to be initialized
    #[account(
        init,
        seeds = [
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &tick.to_be_bytes()
        ],
        bump = tick_account_bump,
        payer = signer
    )]
    pub tick_state: Loader<'info, TickState>,

    /// Program to initialize the tick account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bitmap_account_bump: u8, tick: i32)]
pub struct InitBitmapAccount<'info> {
    /// Pays to create bitmap account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Create a new bitmap account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The bitmap account to be initialized
    #[account(
        init,
        seeds = [
            BITMAP_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &((tick >> 8) as i16).to_be_bytes()
        ],
        bump = bitmap_account_bump,
        payer = signer
    )]
    pub bitmap_state: Loader<'info, TickBitmapState>,

    /// Program to initialize the tick account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitPositionAccount<'info> {
    /// Pays to create position account
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The address of the position owner
    pub recipient: UncheckedAccount<'info>,

    /// Create a position account for this pool
    pub pool_state: Loader<'info, PoolState>,

    /// The lower tick boundary of the position
    pub tick_lower_state: Loader<'info, TickState>,

    /// The upper tick boundary of the position
    #[account(
        constraint = tick_lower_state.load()?.tick < tick_upper_state.load()?.tick @ErrorCode::TLU
    )]
    pub tick_upper_state: Loader<'info, TickState>,

    /// The position account to be initialized
    #[account(
        init,
        seeds = [
            POSITION_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            recipient.key().as_ref(),
            &tick_lower_state.load()?.tick.to_be_bytes(),
            &tick_upper_state.load()?.tick.to_be_bytes(),
        ],
        bump = bump,
        payer = signer
    )]
    pub position_state: Loader<'info, PositionState>,

    /// Program to initialize the position account
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(
    amount: u32
)]
pub struct MintContext<'info> {
    /// Pays to mint liquidity
    #[account(mut)]
    pub minter: Signer<'info>,

    /// Liquidity is minted on behalf of recipient
    pub recipient: UncheckedAccount<'info>,

    /// Mint liquidity for this pool
    #[account(mut)]
    pub pool_state: Loader<'info, PoolState>,

    /// The lower tick boundary of the position
    #[account(
        mut,
        seeds = [
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &tick_lower_state.load()?.tick.to_be_bytes()
        ],
        bump = tick_lower_state.load()?.bump,
    )]
    pub tick_lower_state: Loader<'info, TickState>,

    /// The upper tick boundary of the position
    #[account(
        mut,
        seeds = [
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &tick_upper_state.load()?.tick.to_be_bytes()
        ],
        bump = tick_upper_state.load()?.bump,
    )]
    pub tick_upper_state: Loader<'info, TickState>,

    /// The bitmap storing initialization state of the lower tick
    #[account(
        mut,
        seeds = [
            BITMAP_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &bitmap_lower.load()?.word_pos.to_be_bytes(),
        ],
        bump = bitmap_lower.load()?.bump,
    )]
    pub bitmap_lower: Loader<'info, TickBitmapState>,

    /// The bitmap storing initialization state of the upper tick
    #[account(
        mut,
        seeds = [
            BITMAP_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &bitmap_upper.load()?.word_pos.to_be_bytes(),
        ],
        bump = bitmap_upper.load()?.bump,
    )]
    pub bitmap_upper: Loader<'info, TickBitmapState>,

    /// The position into which liquidity is minted
    #[account(
        mut,
        seeds = [
            POSITION_SEED.as_bytes(),
            pool_state.load()?.token_0.key().as_ref(),
            pool_state.load()?.token_1.key().as_ref(),
            &pool_state.load()?.fee.to_be_bytes(),
            &recipient.key().as_ref(),
            &tick_lower_state.load()?.tick.to_be_bytes(),
            &tick_upper_state.load()?.tick.to_be_bytes(),
        ],
        bump = position_state.load()?.bump,
    )]
    pub position_state: Loader<'info, PositionState>,

    /// The token account spending token_0 to mint the position
    #[account(mut)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,

    /// The token account spending token_1 to mint the position
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_0
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_0.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_0: Box<Account<'info, TokenAccount>>,

    /// The address that holds pool tokens for token_1
    #[account(
        mut,
        associated_token::mint = pool_state.load()?.token_1.key(),
        associated_token::authority = pool_state,
    )]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    /// The SPL program to perform token transfers
    pub token_program: Program<'info, Token>,

    // // // pub callback_handler: Program<'info, NonFungiblePositionManager>,
    // pub system_program: Program<'info, System>,
}


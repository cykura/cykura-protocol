use anchor_lang::prelude::*;
use std::mem::size_of;

use crate::states::factory::FactoryState;
use crate::states::fee::FeeState;
use crate::states::pool::PoolState;
use crate::states::position::PositionState;

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Initialize<'info> {
    pub owner: Signer<'info>,

    #[account(
        init,
        seeds = [],
        bump = bump,
        payer = owner,
        space = size_of::<FactoryState>() + 10
    )]
    pub factory_state: Box<Account<'info, FactoryState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee: u32, tick_spacing: u16, fee_bump: u8)]
pub struct EnableFeeAmount<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [],
        bump = factory_state.bump,
        constraint = owner.key() == factory_state.owner
    )]
    pub factory_state: Box<Account<'info, FactoryState>>,

    #[account(
        init,
        seeds = [&fee.to_be_bytes()],
        bump = fee_bump,
        payer = owner,
        space = size_of::<FeeState>() + 10
    )]
    pub fee_state: Box<Account<'info, FeeState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee: u32, token0: Pubkey, token1: Pubkey, tick_lower:u128, tick_upper:u128, bump: u8)]
pub struct CreatePosition<'info> {
    pub owner: Signer<'info>,

    #[account(
        init,
        seeds = [token0.as_ref(), token1.as_ref(), &fee.to_be_bytes(), &tick_lower.to_be_bytes(), &tick_upper.to_be_bytes()],
        bump = bump,
        payer = owner,
        space = size_of::<FeeState>() + 10
    )]
    pub pool_state: Box<Account<'info, PositionState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(fee: u32, token0: Pubkey, token1: Pubkey, bump: u8)]
pub struct CreatePool<'info> {
    pub owner: Signer<'info>,

    #[account(
        init,
        seeds = [token0.as_ref(), token1.as_ref(), &fee.to_be_bytes()],
        bump = bump,
        payer = owner,
        space = size_of::<FeeState>() + 10
    )]
    pub pool_state: Box<Account<'info, PoolState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct SetOwner<'info> {
    pub owner: Signer<'info>,
    pub new_owner: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [],
        bump = factory_state.bump,
        // TODO error message
        constraint = owner.key() == factory_state.owner
    )]
    pub factory_state: Box<Account<'info, FactoryState>>,
}
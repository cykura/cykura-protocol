use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("37kn8WUzihQoAnhYxueA2BnqCA7VRnrVvYoHy1hQ6Veu");

#[program]
pub mod cyclos_protocol_v2 {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, bump: u8) -> ProgramResult {
        ctx.accounts.factory_state.bump = bump;
        ctx.accounts.factory_state.owner = ctx.accounts.owner.key();

        // TODO : Figure out how to give default Pubkey
        // let pk: [u8;32] = [0; 32];
        // msg!("Default pubkey {}", default_address::ID);

        emit!(OwnerChangedEvent {
            old_owner: ctx.accounts.owner.key(),
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
}

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

#[account]
pub struct FactoryState {
    pub bump: u8,
    pub owner: Pubkey,
}

#[account]
pub struct FeeState {
    pub bump: u8,
    pub fee: u32, 
    pub tick_spacing: u16,
}

// Error Codes
#[error]
pub enum ErrorCode {
    #[msg("Fees collected should be less than 1_000_000 (100%)")]
    FeeLimit,
    #[msg("Tick spacing should be less than 16384")]
    TickSpacingLimit,
}

// Events
#[event]
pub struct OwnerChangedEvent {
    #[index]
    pub old_owner: Pubkey,
    #[index]
    pub new_owner: Pubkey,
}

#[event]
pub struct FeeAmountEnabledEvent {
    #[index]
    pub fee: u32,
    #[index]
    pub tick_spacing: u16,
}

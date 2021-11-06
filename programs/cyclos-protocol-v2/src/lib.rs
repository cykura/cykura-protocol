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

    pub fn create_pool(ctx: Context<Todo>) -> ProgramResult {
        todo!("Unique pool for [tokenA, tokenB, fee] where tokenA > tokenB")
    }

    // ---------------------------------------------------------------------
    // 1. Pool instructions

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


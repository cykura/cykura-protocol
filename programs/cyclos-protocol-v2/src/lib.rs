use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod cyclos_protocol_v2 {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, bump: u8) -> ProgramResult {
        ctx.accounts.factory_state.bump = bump;
        ctx.accounts.factory_state.owner = ctx.accounts.owner.key();

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
        space = size_of::<FactoryState>()
    )]
    pub factory_state: Account<'info, FactoryState>,
    pub system_program: Program<'info, System>,
}


#[account]
pub struct FactoryState {
    pub bump: u8,
    pub owner: Pubkey,
}
use anchor_lang::prelude::*;
use anchor_spl::token::{self, SetAuthority, Token, TokenAccount, Transfer};
// use cyclos_protocol_v2::states::pool::PoolState;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod non_fungible_position_manager {
    use super::*;

    pub fn mint_callback(
        ctx: Context<MintCallback>,
        amount_0_owed: u64,
        amount_1_owed: u64,
        data: [u8; 256],
    ) -> ProgramResult {
        // TODO callback should come from pool contract

        // Transfer tokens from user to core program's vault_0
        if amount_0_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.token_account_0.to_account_info().clone(),
                        to: ctx.accounts.vault_0.to_account_info().clone(),
                        authority: ctx.accounts.minter.to_account_info().clone(),
                    },
                ),
                amount_0_owed,
            )?;
        }
        // Transfer tokens from user to core program's vault_1
        if amount_1_owed > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info().clone(),
                    token::Transfer {
                        from: ctx.accounts.token_account_1.to_account_info().clone(),
                        to: ctx.accounts.vault_1.to_account_info().clone(),
                        authority: ctx.accounts.minter.to_account_info().clone(),
                    },
                ),
                amount_1_owed,
            )?;
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct MintCallback<'info> {
    pub minter: Signer<'info>,

    // Should be a PDA of core contract
    // Core contract (factory in v3) must be passed via a constructor
    #[account(signer)]
    pub pool_state: AccountInfo<'info>,

    #[account(mut)]
    pub token_account_0: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_account_1: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_0: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault_1: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(fee: u32, token0: Pubkey, token1: Pubkey, tick_lower:u128, tick_upper:u128, bump: u8)]
pub struct CreatePosition<'info> {
    pub owner: Signer<'info>,

    #[account(
        init,
        seeds = [&token0, &token1, &fee, &tick_lower, &tick_upper],
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
        seeds = [&token0, &token1, &fee],
        bump = bump,
        payer = owner,
        space = size_of::<FeeState>() + 10
    )]
    pub pool_state: Box<Account<'info, PoolState>>,
    pub system_program: Program<'info, System>,
}
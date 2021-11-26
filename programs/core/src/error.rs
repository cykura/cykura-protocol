use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("Fees collected should be less than 1_000_000 (100%)")]
    FeeLimit,
    #[msg("Tick spacing should be less than 16384")]
    TickSpacingLimit,
    #[msg("Signer is not the state owner")]
    NotAnOwner,
    #[msg("Pool is Locked")]
    Locked,
    #[msg("Invalid associated token account")]
    NotAssociatedTokenAccount,
    #[msg("Minting amount should be greater than 0")]
    ZeroMintAmount,

    // Balance in pool before minting should be less than or equal to balance before minting
    #[msg("M0")]
    M0,
    #[msg("M1")]
    M1,

    // libraries/tick_math.rs

    // second inequality must be < because the price can never reach the price at the max tick
    #[msg("R")]
    R,
    // The given tick must be less than, or equal to, the maximum tick
    #[msg("T")]
    T,

    // libraries/liquidity_math.rs

    #[msg("LS")] // Liquidity Sub
    LS,

    #[msg("LA")] // Liquidity Add
    LA,
}

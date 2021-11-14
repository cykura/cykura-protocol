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
    M1
}

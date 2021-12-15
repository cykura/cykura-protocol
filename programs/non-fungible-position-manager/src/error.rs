use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("Transaction too old")]
    TransactionTooOld,

    #[msg("Price slippage check")]
    PriceSlippageCheck,

    #[msg("Not approved")]
    NotApproved
}

use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("Transaction too old")]
    TransactionTooOld,
}

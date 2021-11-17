use anchor_lang::prelude::*;

#[error]
pub enum ErrorCode {
    #[msg("Transaction is Older than blockhash")]
    OldTransaction,
}

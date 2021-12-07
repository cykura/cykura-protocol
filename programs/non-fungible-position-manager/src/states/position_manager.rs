use anchor_lang::prelude::*;

#[account(zero_copy)]
#[derive(Default)]
pub struct PositionManagerState {
    /// Bump to identify PDA
    pub bump: u8,
}

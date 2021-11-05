use anchor_lang::prelude::*;

#[account]
pub struct FactoryState {
    pub bump: u8,
    pub owner: Pubkey,
}

#[event]
pub struct OwnerChangedEvent {
    #[index]
    pub old_owner: Pubkey,
    #[index]
    pub new_owner: Pubkey,
}
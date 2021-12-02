use anchor_lang::prelude::*;

/// Holds the current owner of the factory
///
/// # The owner can
///
/// 1. Set and collect a pool's protocol fees
/// 2. Enable a new fee amount for pool creation
/// 3. Set another address as an owner
///
/// PDA of `[]`
///
#[account]
pub struct FactoryState {
    /// Bump to identify PDA
    pub bump: u8,

    /// Address of the protocol owner
    pub owner: Pubkey,
}

/// Emitted when the owner of the factory is changed
#[event]
pub struct OwnerChanged {
    /// The owner before the owner was changed
    #[index]
    pub old_owner: Pubkey,

    /// The owner after the owner was changed
    #[index]
    pub new_owner: Pubkey,
}

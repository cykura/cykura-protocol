use anchor_lang::prelude::*;

#[account]
pub struct TickBitmap {
    pub bump: u8,
    pub bitMap: [bool; 256],
}

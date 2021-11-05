/// Track whether valid ticks are initialized or not
/// A tick is valid if it is a multiple of tick_spacing
/// Each bitmap account stores data for 2^8 (256) ticks
/// Ticks are in i24 format. The first 16 bits go in the PDA, while remaining
/// 8 bits are tracked by the bitmap

use anchor_lang::prelude::*;
use ux::i24;

#[account]
pub struct TickBitmap {
    pub bump: u8,
    pub bit_map: [bool; 256],
}

impl TickBitmap {
    // Flip tick if it's a multiple of spacing, else panic
    pub fn flip_tick(&mut self, tick: i24, tick_spacing: i24) {
        todo!()
    }

    // Get next initialized tick in given word
    // Look to the left if less than or equal (lte) is true, else look at right
    // Modification: use right bits instead of entire tick. Left bits are used
    // to find PDA
    // Use simple looping. Mask bitwise logic avoided due to use of bool array
    // TODO will [bool; 256] hit stack size limit?
    pub fn next_initialized_tick_within_one_word(
        &self,
        tick_right_bits: u8,
        lte: bool
    ) -> (i8, bool) {
        todo!()
    }
}

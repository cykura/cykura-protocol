/// Track whether valid ticks are initialized or not
/// A tick is valid if it is a multiple of tick_spacing
/// Each bitmap account stores data for 2^8 (256) ticks
/// Ticks are in i24 format. The first 16 bits go in the PDA, while remaining
/// 8 bits are tracked by the bitmap

use anchor_lang::prelude::*;
use ux::i24;
// use bitmaps::Bitmap;

// addr: [token0, token1, fee, 16_bits_from_left(tick)]
#[account]
pub struct TickBitmapState {
    pub bump: u8,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: Pubkey,

    pub left_16_bits_of_tick: u16,
    pub bit_map: [bool; 256],
}

/// Get tick key and bit position for a tick
/// 24 bits = 16 (key) + 8 (=256)
/// 32 bits = 8 bits (discard) + 16 (key) + 8 (=256)
/// 
/// | [sign][----8 bit waste---][---15 bit word_pos---][---8 bit for bit_pos---] |
pub fn position(tick_div_spacing: i32) -> (i16, i8) {
    assert!(tick_div_spacing >= -429772 && tick_div_spacing <= 429772);

    // right shift: remove rightmost 8 bits
    // modulo 2^15: remove leftmost 9 bits to get 15 bit unsigned word
    // add signed bit if negative. Negative integers have MSB = 1, positive have 0
    let mut word_pos = ((tick_div_spacing >> 8) % 2^15) as i16;
    if tick_div_spacing.is_negative() {
        word_pos = -word_pos;
    }
    
    // bit position is given by rightmost 8 bits
    let bit_pos = tick_div_spacing % 2^8;

    (word_pos, bit_pos)
}

impl TickBitmapState {
    // Flip tick if it's a multiple of spacing, else panic
    // Find the tick to be flipped from client side
    // Check where the tick lives in the word array and flip
    pub fn flip_tick(&mut self, bit_pos: u8) {
        self.bit_map[bit_pos] = !self.bit_map[bit_pos];

        // let mut bitmap = Bitmap::<256>::new();

        // let bitmap_as_num = bitmap
    }
    // pub fn flip_tick(&mut self, tick: i32, tick_spacing: i32) {
    //     assert!(tick % tick_spacing == 0, "Tick is not a multiple of Tick Spacing");

    //     let (word_pos, bit_pos) = position(tick / tick_spacing);

    //     self.bit_map = [];
    //     todo!()
    // }
    

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

#[cfg(test)]
mod tests {
    use super::*;

}

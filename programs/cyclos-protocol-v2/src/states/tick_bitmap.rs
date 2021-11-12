/// Track whether valid ticks are initialized or not
/// A tick is valid if it is a multiple of tick_spacing
/// Each bitmap account stores data for 2^8 (256) ticks
/// Ticks are in i24 format. The first 16 bits go in the PDA, while remaining
/// 8 bits are tracked by the bitmap
use anchor_lang::prelude::*;
// use ux::i24;
use bitmaps::Bitmap;

// addr: [token0, token1, fee, 16_bits_from_left(tick)]
#[account]
pub struct TickBitmapState {
    pub bump: u8,
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub fee: Pubkey,
    pub left_16_bits_of_tick: u16,
    pub bitmap: [u128; 2],
}

// Get tick / spacing
// Tick should be a multiple of spacing
pub fn get_tick_div_spacing(tick: i32, spacing: i32) -> i32 {
    assert_eq!(tick % spacing, 0);
    tick / spacing
}

/// Get tick key and bit position for a tick/spacing
/// 24 bits = 16 (key) + 8 (=256)
/// 32 bits = 8 bits (discard) + 16 (key) + 8 (=256)
///
/// | [sign][----8 bit waste---][---15 bit word_pos---][---8 bit for bit_pos---] |
pub fn get_word_and_bit_pos(tick_div_spacing: i32) -> (i16, u8) {
    assert!(tick_div_spacing >= -429772 && tick_div_spacing <= 429772);

    // right shift: remove rightmost 8 bits
    // modulo 2^15: remove leftmost 9 bits to get 15 bit unsigned word
    // add signed bit if negative. Negative integers have MSB = 1, positive have 0
    let mut word_pos = ((tick_div_spacing >> 8) % 2 ^ 15) as i16;
    if tick_div_spacing.is_negative() {
        word_pos = -word_pos;
    }

    // bit position is given by rightmost 8 bits
    let bit_pos = (tick_div_spacing.abs() % 2 ^ 8) as u8;

    (word_pos, bit_pos)
}

impl TickBitmapState {
    pub fn decode_bitmap(&self) -> bitmaps::Bitmap<256> {
        Bitmap::<256>::from(self.bitmap)
    }

    // Flip tick if it's a multiple of spacing, else panic
    // Find the tick to be flipped from client side
    // Check where the tick lives in the word array and flip
    // TODO externally find bit_pos using tick: i32, and impose tick % tick_spacing condition
    pub fn flip_tick(&mut self, bit_pos: u8) {
        let mut bitmap = self.decode_bitmap();
        bitmap.set(bit_pos as usize, !bitmap.get(bit_pos as usize));
        self.bitmap = *bitmap.as_value();
    }

    // Get next initialized tick in given word
    // Look to the left if less than or equal (lte) is true, else look at right
    // Modification: use right bits instead of entire tick. Left bits are used
    // to find PDA
    // Use simple looping for now
    // TODO explore mask to remove looping
    // TODO externally find bit_pos using tick: i32, and impose tick % tick_spacing condition
    // Returns bit position of next tick, and whether it is initialized
    pub fn next_initialized_tick_within_one_word(
        &self,
        current_bit_pos: u8,
        lte: bool,
    ) -> (u8, bool) {
        let bitmap = self.decode_bitmap();

        if lte {
            // check to the left
            for i in current_bit_pos..=0 {
                let tick = bitmap.get(i as usize);
                if tick == true {
                    return (i, true);
                }
            }
            (0, false)
        } else {
            for i in (current_bit_pos + 1)..(bitmap.len() as u8) {
                let tick = bitmap.get(i as usize);
                if tick == true {
                    return (i, true);
                }
            }
            (bitmap.len() as u8 - 1, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bitmap_test() {
        let mut bitmap = Bitmap::<256>::new();
        println!("BItmap length {}", bitmap.len());

        bitmap.set(0, true);
        let converted_bitmap = bitmap.as_value() as &[u128; 2];
        msg!("Converted bitmap {:?}", converted_bitmap);

        let arr = [10_u128, 50];
        let decoded_bitmap = Bitmap::<256>::from(arr);
        msg!("Decoded bitmap {:?}", decoded_bitmap.into_value());
    }
}

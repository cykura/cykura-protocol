///! Packed tick initialized state library
///! Stores a packed mapping of tick index to its initialized state
///
///! Although ticks are stored as i32, all tick values fit within 24 bits.
///! Therefore the mapping uses i16 for keys and there are 256 (2^8) values per word.
///!

use std::ops::BitOr;
use std::ops::BitXor;
use std::ops::BitXorAssign;
use anchor_lang::prelude::*;
use bitmaps::Bitmap;
use bitmaps::Bits;
use bitmaps::BitsImpl;
use crate::libraries::bit_math;
use crate::libraries::big_num::U256;

/// Seed to derive account address and signature
pub const BITMAP_SEED: &str = "b";

/// Stores info for a single bitmap word.
/// Each word represents 256 packed tick initialized boolean values.
///
/// Emulates a solidity mapping, where word_position is the key and
///
/// PDA of `[BITMAP_SEED, token_0, token_1, fee, word_pos]`
///
#[account(zero_copy)]
#[derive(Default)]
pub struct TickBitmapState {
    /// Bump to identify PDA
    pub bump: u8,

    /// The bitmap key. To find word position from a tick, divide the tick by tick spacing
    /// to get a 24 bit compressed result, then right shift to obtain the most significant 16 bits.
    pub word_pos: i16,

    /// Packed initialized state
    pub word: [u64; 4],
}

/// The position in the mapping where the initialized bit for a tick lives
pub struct Position {
    /// The key in the mapping containing the word in which the bit is stored
    pub word_pos: i16,

    /// The bit position in the word where the flag is stored
    pub bit_pos: u8
}

/// Computes the position in the mapping where the initialized bit for a tick lives.
///
///
/// # Arguments
///
/// * `tick_by_spacing` - The tick for which to compute the position, divided by pool tick spacing
///
pub fn position(tick_by_spacing: i32) -> Position {
    Position {
        word_pos: (tick_by_spacing >> 8) as i16,
        // begins with 255 for negative numbers
        bit_pos: (tick_by_spacing % 256) as u8
    }
}

// // Get tick / spacing
// // Tick should be a multiple of spacing
// pub fn get_tick_div_spacing(tick: i32, spacing: i32) -> i32 {
//     assert_eq!(tick % spacing, 0);
//     tick / spacing
// }

// /// Get tick key and bit position for a tick/spacing value
// /// 24 bits = 16 (key) + 8 (=256)
// /// 32 bits = 8 bits (discard) + 16 (key) + 8 (=256)
// ///
// /// | [sign][----8 bit waste---][---15 bit word_pos---][---8 bit for bit_pos---] |
// pub fn get_word_and_bit_pos(tick_div_spacing: i32) -> (i16, u8) {
//     assert!(tick_div_spacing >= -429772 && tick_div_spacing <= 429772);

//     // right shift: remove rightmost 8 bits
//     // modulo 2^15: remove leftmost 9 bits to get 15 bit unsigned word
//     let mut word_pos = ((tick_div_spacing >> 8) % 2 ^ 15) as i16;

//     // add signed bit if negative. Negative integers have MSB = 1, positive have 0
//     if tick_div_spacing.is_negative() {
//         word_pos = -word_pos;
//     }

//     // bit position is given by rightmost 8 bits
//     let bit_pos = (tick_div_spacing.abs() % 2 ^ 8) as u8;

//     (word_pos, bit_pos)
// }

impl TickBitmapState {

    ///  Flips the initialized state for a given tick from false to true, or vice versa
    ///
    /// # Arguments
    ///
    /// * `self` - The bitmap state corresponding to the tick's word position
    /// * `bit_pos` - The rightmost 8 bits of the tick
    ///
    pub fn flip_tick(&mut self, bit_pos: u8) {
        let word = U256(self.word);
        let mask = U256::default() << bit_pos;
        self.word = word.bitxor(mask).0;
    }

    /// Returns the bit position for the next initialized tick contained in the same word (or adjacent word)
    /// as the tick that is either to the left (less than or equal to) or right (greater than) of the given tick
    ///
    /// # Arguments
    ///
    /// * `self` - The mapping in which to compute the next initialized tick
    /// * `bit_pos` - The starting bit position
    /// * `lte` - Whether to search for the next initialized tick to the left (less than or equal to the starting tick)
    ///
    pub fn next_initialized_bit(
        &self,
        bit_pos: u8,
        lte: bool
    ) -> u8 {
        let word = U256(self.word);
        if lte {
            // all the 1s at or to the right of the current bitPos
            let mask = (U256::from(1) << bit_pos) - 1 + (U256::from(1) << bit_pos);
            let masked = word & mask;

            let initialized = mask != U256::default();

            // masked.leading_zeros()
            // let next = if initialized {
            //     bit_pos - bit_math::most_significant_bit(masked)
            // }
        }
        1
    }

    // Get next initialized tick in given word
    // Look to the left if less than or equal (lte) is true, else look at right
    // Modification: use right bits instead of entire tick. Left bits are used
    // to find PDA
    // Use simple looping for now
    // TODO explore mask to remove looping
    // TODO externally find bit_pos using tick: i32, and impose tick % tick_spacing condition
    // Returns bit position of next tick, and whether it is initialized
    // pub fn next_initialized_tick_within_one_word(
    //     &self,
    //     current_bit_pos: u8,
    //     lte: bool,
    // ) -> (u8, bool) {
    //     let bitmap = self.decode_bitmap();

    //     if lte {
    //         // check to the left
    //         for i in current_bit_pos..=0 {
    //             let tick = bitmap.get(i as usize);
    //             if tick == true {
    //                 return (i, true);
    //             }
    //         }
    //         (0, false)
    //     } else {
    //         for i in (current_bit_pos + 1)..(bitmap.len() as u8) {
    //             let tick = bitmap.get(i as usize);
    //             if tick == true {
    //                 return (i, true);
    //             }
    //         }
    //         (bitmap.len() as u8 - 1, false)
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_for_negative_tick() {
        let pos = position(-1);
        assert_eq!(pos.word_pos, -1);
        assert_eq!(pos.bit_pos, 255);
    }

    #[test]
    fn msb_lsb_test() {
        let a = U256::from(1);
        println!("leading zeroes {}", a.leading_zeros());
        println!("trailing zeroes {}", a.trailing_zeros());
    }
    // #[test]
    // fn bitmap_test() {
    //     let mut bitmap = Bitmap::<256>::new();
    //     println!("Bitmap length {}", bitmap.len());

    //     bitmap.set(0, true);
    //     let converted_bitmap = bitmap.as_value() as &[u128; 2];
    //     msg!("Converted bitmap {:?}", converted_bitmap);

    //     let arr = [10_u128, 50];
    //     let decoded_bitmap = Bitmap::<256>::from(arr);
    //     msg!("Decoded bitmap {:?}", decoded_bitmap.into_value());
    // }

    // #[test]
    // fn flip_start_of_first_item() {
    //     let mut tb_state = BitmapState::default();

    //     tb_state.flip_tick(0);
    //     assert_eq!(tb_state.bitmap, [1, 0]);

    //     tb_state.flip_tick(0);
    //     assert_eq!(tb_state.bitmap, [0, 0]);
    // }

    // #[test]
    // fn flip_end_of_first_item() {
    //     let mut tb_state = BitmapState::default();
    //     tb_state.flip_tick(127);
    //     assert_eq!(tb_state.bitmap, [u128::pow(2, 127), 0]);
    //     tb_state.flip_tick(127);
    //     assert_eq!(tb_state.bitmap, [0, 0]);
    // }

    // #[test]
    // fn flip_start_of_second_item() {
    //     let mut tb_state = BitmapState::default();
    //     tb_state.flip_tick(128);
    //     assert_eq!(tb_state.bitmap, [0, 1]);
    //     tb_state.flip_tick(128);
    //     assert_eq!(tb_state.bitmap, [0, 0]);
    // }

    // #[test]
    // fn flip_end_of_second_item() {
    //     let mut tb_state = BitmapState::default();
    //     tb_state.flip_tick(255);
    //     assert_eq!(tb_state.bitmap, [0, u128::pow(2, 127)]);
    //     tb_state.flip_tick(255);
    //     assert_eq!(tb_state.bitmap, [0, 0]);
    // }
}

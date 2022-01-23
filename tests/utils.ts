import BN from "bn.js";

export const MIN_SQRT_RATIO = new BN(65536)
export const MAX_SQRT_RATIO = new BN(281474976710656)

export const MIN_TICK = -221818
export const MAX_TICK = 221818

// Generate seed buffer from a u32 number
export function u32ToSeed(num: number) {
  const arr = new ArrayBuffer(4)
  const view = new DataView(arr)
  view.setUint32(0, num, false)
  return new Uint8Array(arr)
}

export function i32ToSeed(num: number) {
  const arr = new ArrayBuffer(4)
  const view = new DataView(arr)
  view.setInt32(0, num, false)
  return new Uint8Array(arr)
}

// Generate seed buffer from a u32 number
export function u16ToSeed(num: number) {
  const arr = new ArrayBuffer(2)
  const view = new DataView(arr)
  view.setUint16(0, num, false)
  return new Uint8Array(arr)
}

export function i16ToSeed(num: number) {
  const arr = new ArrayBuffer(2)
  const view = new DataView(arr)
  view.setInt16(0, num, false)
  return new Uint8Array(arr)
}

export const MaxU64= new BN(2).pow(new BN(64)).subn(1)

// Seed bumps
export const BITMAP_SEED = Buffer.from('b')
export const POOL_SEED = Buffer.from('p')
export const POSITION_SEED = Buffer.from('ps')
export const OBSERVATION_SEED = Buffer.from('o')
export const TICK_SEED = Buffer.from('t')
export const FEE_SEED = Buffer.from('f')

export function generateBitmapWord(x: BN[]) {
  return x[0]
    .add(x[1].shln(64))
    .add(x[2].shln(126))
    .add(x[3].shln(192))
}

export function mostSignificantBit(x: BN) {
  return x.bitLength() - 1
}

export function leastSignificantBit(x: BN) {
  return x.zeroBits()
}

export type NextBit = {
  next: number,
  initialized: boolean,
}

/**
 * Returns the bitmap index (0 - 255) for the next initialized tick.
 * 
 * If no initialized tick is available, returns the first bit (index 0) the word in lte case,
 * and the last bit in gte case.
 * @param word The bitmap word as a u256 number
 * @param bitPos The starting bit position
 * @param lte Whether to search for the next initialized tick to the left (less than or equal to the starting tick),
 * or to the right (greater than or equal to)
 * @returns Bit index and whether it is initialized
 */
export function nextInitializedBit(word: BN, bitPos: number, lte: boolean): NextBit {
  if (lte) {
    // all the 1s at or to the right of the current bit_pos
    const mask = new BN(1).shln(bitPos).subn(1).add(new BN(1).shln(bitPos))
    const masked = word.and(mask)
    const initialized = !masked.eqn(0)
    const next = initialized
      ? mostSignificantBit(masked)
      : 0
    return { next, initialized }
  } else {
    // all the 1s at or to the left of the bit_pos
    const mask = new BN(1).shln(bitPos).subn(1).notn(256)
    const masked = word.and(mask)
    const initialized = !masked.eqn(0)
    const next = initialized
      ? mostSignificantBit(masked)
      : 255
    return { next, initialized }
  }
}

export type Position = {
  wordPos: number,
  bitPos: number
}

/**
 * Computes the bitmap position for a bit.
 * @param tickBySpacing Tick divided by spacing
 * @returns the word and bit position for the given tick
 */
export function tickPosition(tickBySpacing: number): Position {
  return {
    wordPos: tickBySpacing >> 8,
    bitPos: Math.abs(tickBySpacing % 256),
  }
}
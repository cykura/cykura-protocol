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
 * Get the next initialized bit in the bitmap
 * @param word 
 * @param bitPos 
 * @param lte 
 * @returns 
 */
export function nextInitializedBit(word: BN, bitPos: number, lte: boolean): NextBit {
  const nextBit: NextBit = {
    next: 0,
    initialized: false,
  }
  if (lte) {
    const mask = new BN(1).shln(bitPos).subn(1).add(new BN(1).shln(bitPos))
    const masked = word.and(mask)
    nextBit.initialized = !masked.eqn(0)
    nextBit.next = nextBit.initialized
      ? mostSignificantBit(masked) - bitPos
      : - bitPos
  } else {
    // TODO
  }
  return nextBit
}
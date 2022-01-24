import { TickDataProvider } from "@uniswap/v3-sdk"
import * as anchor from '@project-serum/anchor'
import { CyclosCore } from '../target/types/cyclos_core'
import BN from "bn.js"
import JSBI from "jsbi"
import { BigintIsh } from "@uniswap/sdk-core"
import { PublicKey } from "@solana/web3.js"
import { existsSync } from "fs"

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

export const MaxU64 = new BN(2).pow(new BN(64)).subn(1)

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

export type PoolVars = {
  token0: PublicKey,
  token1: PublicKey,
  fee: number,
}
export class SolanaTickDataProvider implements TickDataProvider {
  program: anchor.Program<CyclosCore>
  pool: PoolVars

  constructor(program: anchor.Program<CyclosCore>, pool: PoolVars) {
    this.program = program
    this.pool = pool
  }

  async getTick(tick: number): Promise<{ liquidityNet: BigintIsh }> {
    const tickState = (await PublicKey.findProgramAddress([
      TICK_SEED,
      this.pool.token0.toBuffer(),
      this.pool.token1.toBuffer(),
      u32ToSeed(this.pool.fee),
      u32ToSeed(tick)
    ],
      this.program.programId
    ))[0]

    const { liquidityNet } = await this.program.account.tickState.fetch(tickState)
    return {
      liquidityNet: JSBI.BigInt(liquidityNet),
    }
  }

  async nextInitializedTickWithinOneWord(tick: number, lte: boolean, tickSpacing: number): Promise<[number, boolean]> {
    // TODO optimize function. Currently bitmaps are repeatedly fetched, even if two ticks are on the same bitmap
    let compressed = Math.floor(tick / tickSpacing)
    if (!lte) {
      compressed += 1
    }

    const { wordPos, bitPos } = tickPosition(compressed)

    const bitmapState = (await PublicKey.findProgramAddress([
      BITMAP_SEED,
      this.pool.token0.toBuffer(),
      this.pool.token1.toBuffer(),
      u32ToSeed(this.pool.fee),
      u16ToSeed(wordPos),
    ], this.program.programId))[0]

    let nextBit = lte ? 0 : 255
    let initialized = false
    try {
      const { word: wordArray } = await this.program.account.tickBitmapState.fetch(bitmapState)
      const word = generateBitmapWord(wordArray)
      const nextInitBit = nextInitializedBit(word, bitPos, lte)
      nextBit = nextInitBit.next
      initialized = nextInitBit.initialized
    } catch(error) {
      console.log('bitmap account doesnt exist, using defaults')
    }
    const nextTick = (wordPos * 256 + nextBit) * tickSpacing
    return [nextTick, initialized]
    
  }
}
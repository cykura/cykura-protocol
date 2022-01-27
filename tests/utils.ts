import {
  TickDataProvider, PoolVars, tickPosition, generateBitmapWord, nextInitializedBit, u32ToSeed,
  TICK_SEED,
  BITMAP_SEED,
  u16ToSeed
} from "@uniswap/v3-sdk"
import * as anchor from '@project-serum/anchor'
import { CyclosCore } from '../target/types/cyclos_core'
import BN from "bn.js"
import { BigintIsh } from "@uniswap/sdk-core"
import { PublicKey } from "@solana/web3.js"

export const MIN_SQRT_RATIO = new BN(65536)
export const MAX_SQRT_RATIO = new BN(281474976710656)

export const MIN_TICK = -221818
export const MAX_TICK = 221818

export const MaxU64 = new BN(2).pow(new BN(64)).subn(1)

export class SolanaTickDataProvider implements TickDataProvider {
  program: anchor.Program<CyclosCore>
  pool: PoolVars

  constructor(program: anchor.Program<CyclosCore>, pool: PoolVars) {
    this.program = program
    this.pool = pool
  }

  async getTick(tick: number): Promise<{ liquidityNet: BigintIsh; }> {
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
      liquidityNet: liquidityNet.toString(),
    }
  }

  async getTickAddress(tick: number): Promise<anchor.web3.PublicKey> {
    return (await PublicKey.findProgramAddress([
      TICK_SEED,
      this.pool.token0.toBuffer(),
      this.pool.token1.toBuffer(),
      u32ToSeed(this.pool.fee),
      u32ToSeed(tick)
    ], this.program.programId))[0]
  }

  async nextInitializedTickWithinOneWord(tick: number, lte: boolean, tickSpacing: number)
    : Promise<[number, boolean, number, number, PublicKey]> {
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
    } catch (error) {
      console.log('bitmap account doesnt exist, using default nextbit', nextBit)
    }
    const nextTick = (wordPos * 256 + nextBit) * tickSpacing
    console.log('returning next tick', nextTick)
    return [nextTick, initialized, wordPos, bitPos, bitmapState]
  }
}

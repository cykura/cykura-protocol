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

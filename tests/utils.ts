import BN from "bn.js";

export const MIN_SQRT_RATIO = new BN(65536)
export const MAX_SQRT_RATIO = new BN(281474976710656)

// Generate seed buffer from a u32 number
export function u32ToSeed(num: number) {
  const arr = new ArrayBuffer(4)
  const view = new DataView(arr)
  view.setUint32(0, num, false)
  return new Uint8Array(arr)
}

// Generate seed buffer from a u32 number
export function u16ToSeed(num: number) {
  const arr = new ArrayBuffer(2)
  const view = new DataView(arr)
  view.setUint16(0, num, false)
  return new Uint8Array(arr)
}

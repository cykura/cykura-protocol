import { BN } from "@project-serum/anchor"
import { assert } from 'chai'
import { nextInitializedBit } from "./utils"

describe('bit test', () => {
    describe('lte is true', () => {
        it('return same bit if initialized', () => {
            const bitmap = new BN(1) // ..001
            const currentPos = 0 // initialized bit at position 0
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, true)
            assert.equal(next, currentPos)
            assert(initialized)
        })

        it('returns bit directly to the left of input bit if not initialized', () => {
            const bitmap = new BN(1) // ..001
            const currentPos = 1 // uninitialized 
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, true)
            assert.equal(next, 0)
            assert(initialized)
        })

        it('does not exceed lower boundary if no initialized bit', () => {
            const bitmap = new BN(0)
            const currentPos = 1 // uninitialized 
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, true)
            assert.equal(next, 0)
            assert(!initialized)
        })

        it('when the current bit is initialized', () => {
            let bitmap = new BN(3) // ..011
            const currentPos = 1 // initialized
            const next = nextInitializedBit(bitmap, currentPos, true)
            console.log('next', next)
        })

        it('when no bit is initialized', () => {
            let bitmap = new BN(0) // ..000
            const currentPos = 1 // uninitialized
            const next = nextInitializedBit(bitmap, currentPos, true)
            console.log('next', next)
        })
    })

    describe('lte is false', () => {
        it('returns same bit if initialized', () => {
            const bitmap = new BN(1) // ..001
            const currentPos = 0 // initialized bit at position 0
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, false)
            assert.equal(next, currentPos)
            assert(initialized)
        })

        it('returns bit at right if at uninitialized bit', () => {
            const bitmap = new BN(2) // ..010
            const currentPos = 0 // initialized bit at position 0
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, false)
            assert.equal(next, 1)
            assert(initialized)
        })

        it('does not exceed boundary if no initialized bit', () => {
            const bitmap = new BN(0) // ..000
            const currentPos = 0 // initialized bit at position 0
            const { next, initialized } = nextInitializedBit(bitmap, currentPos, false)
            assert.equal(next, 255)
            assert(!initialized)
        })
    })
})
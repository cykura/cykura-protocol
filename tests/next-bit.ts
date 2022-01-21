import { BN } from "@project-serum/anchor"
import { nextInitializedBit } from "./utils"

describe('bit test', () => {
    describe('less than or equal to the current bit', () => {
        it('an initialized bit less than current exists', async () => {
            const bitmap = new BN(1) // ..001
            const currentPos = 1 // uninitialized 
            const next = nextInitializedBit(bitmap, currentPos, true)
            console.log('next', next)
        })

        it('when the current bit is initialized', async () => {
            let bitmap = new BN(3) // ..011
            const currentPos = 1 // initialized
            const next = nextInitializedBit(bitmap, currentPos, true)
            console.log('next', next)
        })

        it('when no bit is initialized', async () => {
            let bitmap = new BN(0) // ..000
            const currentPos = 1 // uninitialized
            const next = nextInitializedBit(bitmap, currentPos, true)
            console.log('next', next)
        })
    })
})
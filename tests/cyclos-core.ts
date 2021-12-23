import * as anchor from '@project-serum/anchor'
import { Program, web3, BN, ProgramError } from '@project-serum/anchor'
import * as metaplex from '@metaplex/js'
import { Token, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { assert, expect } from 'chai'
import * as chai from 'chai'
import chaiAsPromised from 'chai-as-promised'
chai.use(chaiAsPromised)

import { CyclosCore } from '../target/types/cyclos_core'
import { NonFungiblePositionManager } from '../target/types/non_fungible_position_manager'
import { SwapRouter } from '../target/types/swap_router'
import {
  BITMAP_SEED,
  FEE_SEED,
  MaxU64,
  MAX_SQRT_RATIO,
  MAX_TICK,
  MIN_SQRT_RATIO,
  MIN_TICK,
  OBSERVATION_SEED,
  POOL_SEED,
  POSITION_SEED,
  TICK_SEED,
  u16ToSeed,
  u32ToSeed
} from './utils'

const { metadata: { Metadata } } = metaplex.programs

const { PublicKey, Keypair, SystemProgram } = anchor.web3

describe('cyclos-core', async () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const coreProgram = anchor.workspace.CyclosCore as Program<CyclosCore>
  const mgrProgram = anchor.workspace.NonFungiblePositionManager as Program<NonFungiblePositionManager>
  const routerProgram = anchor.workspace.SwapRouter as Program<SwapRouter>
  const { connection, wallet } = anchor.getProvider()
  const owner = anchor.getProvider().wallet.publicKey
  const notOwner = new Keypair()

  const fee = 500;
  const tickSpacing = 10;

  const [factoryState, factoryStateBump] = await PublicKey.findProgramAddress([], coreProgram.programId);
  const [posMgrState, posMgrBump] = await PublicKey.findProgramAddress([], mgrProgram.programId)

  const [feeState, feeStateBump] = await PublicKey.findProgramAddress(
    [FEE_SEED, u32ToSeed(fee)],
    coreProgram.programId
  );
  console.log("Fee", feeState.toString(), feeStateBump)

  const mintAuthority = new Keypair()

  // Tokens constituting the pool
  let token0: Token
  let token1: Token

  // ATAs to hold pool tokens
  let vault0: web3.PublicKey
  let vault1: web3.PublicKey

  let poolState: web3.PublicKey
  let poolStateBump: number

  let initialObservationState: web3.PublicKey
  let initialObservationBump: number

  // These accounts will spend tokens to mint the position
  let minterWallet0: web3.PublicKey
  let minterWallet1: web3.PublicKey

  let temporaryNftHolder: web3.PublicKey

  const tickLower = 0
  const tickUpper = 10
  const wordPosLower = (tickLower / tickSpacing) >> 8
  const wordPosUpper = (tickUpper / tickSpacing) >> 8

  const amount0Desired = new BN(0)
  const amount1Desired = new BN(1_000_000)
  const amount0Minimum = new BN(0)
  const amount1Minimum = new BN(1_000_000)

  const nftMintKeypair = new Keypair()
  let tickLowerState: web3.PublicKey
  let tickLowerStateBump: number
  let tickUpperState: web3.PublicKey
  let tickUpperStateBump: number
  let corePositionState: web3.PublicKey
  let corePositionBump: number
  let bitmapLower: web3.PublicKey
  let bitmapLowerBump: number
  let bitmapUpper: web3.PublicKey
  let bitmapUpperBump: number
  let tokenizedPositionState: web3.PublicKey
  let tokenizedPositionBump: number
  let positionNftAccount: web3.PublicKey
  let metadataAccount: web3.PublicKey
  let latestObservationState: web3.PublicKey
  let nextObservationState: web3.PublicKey

  const protocolFeeRecipient = new Keypair()
  let feeRecipientWallet0: web3.PublicKey
  let feeRecipientWallet1: web3.PublicKey

  const initialPriceX32 = new BN(4297115210)
  const initialTick = 10

  it('Create token mints', async () => {
    const transferSolTx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: mintAuthority.publicKey,
        lamports: web3.LAMPORTS_PER_SOL,
      })
    )
    transferSolTx.add(
      web3.SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: notOwner.publicKey,
        lamports: web3.LAMPORTS_PER_SOL,
      })
    )
    await anchor.getProvider().send(transferSolTx)

    token0 = await Token.createMint(
      connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      8,
      TOKEN_PROGRAM_ID
    )
    token1 = await Token.createMint(
      connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      8,
      TOKEN_PROGRAM_ID
    )

    console.log('Token 0', token0.publicKey.toString())
    console.log('Token 1', token1.publicKey.toString())

    if (token0.publicKey.toString() > token1.publicKey.toString()) { // swap token mints
      console.log('Swap tokens')
      const temp = token0
      token0 = token1
      token1 = temp
    }
  })

  it('creates token accounts for position minter and airdrops to them', async () => {
    minterWallet0 = await token0.createAssociatedTokenAccount(owner)
    minterWallet1 = await token1.createAssociatedTokenAccount(owner)
    await token0.mintTo(minterWallet0, mintAuthority, [], 100_000_000)
    await token1.mintTo(minterWallet1, mintAuthority, [], 100_000_000)
  })

  it('derive pool address', async () => {
    [poolState, poolStateBump] = await PublicKey.findProgramAddress(
      [
        POOL_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee)
      ],
      coreProgram.programId
    )
    console.log('got pool address', poolState)
  })

  it('derive vault addresses', async () => {
    vault0 = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      token0.publicKey,
      poolState,
      true
    )
    vault1 = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      token1.publicKey,
      poolState,
      true
    )
  })

  describe('#init_factory', () => {

    // Test for event and owner value
    it('initializes factory and emits an event', async () => {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("OwnerChanged", (event, slot) => {
          assert((event.oldOwner as web3.PublicKey).equals(new PublicKey(0)))
          assert((event.newOwner as web3.PublicKey).equals(owner))

          resolve([event, slot]);
        });

        coreProgram.rpc.initFactory(factoryStateBump, {
          accounts: {
            owner,
            factoryState,
            systemProgram: SystemProgram.programId,
          }
        });
      });
      await coreProgram.removeEventListener(listener);

      const factoryStateData = await coreProgram.account.factoryState.fetch(factoryState)
      assert.equal(factoryStateData.bump, factoryStateBump)
      assert(factoryStateData.owner.equals(owner))
    });

    it('Trying to re-initialize factory fails', async () => {
      await expect(coreProgram.rpc.initFactory(factoryStateBump, {
        accounts: {
          owner,
          factoryState,
          systemProgram: anchor.web3.SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    });
  })

  describe('#set_owner', () => {
    const newOwner = new Keypair()

    it('fails if owner does not sign', async () => {
      const tx = coreProgram.transaction.setOwner({
        accounts: {
          owner,
          newOwner: newOwner.publicKey,
          factoryState,
        }
      });
      tx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash

      await expect(connection.sendTransaction(tx, [])).to.be.rejectedWith(Error)
    })

    it('fails if caller is not owner', async () => {
      const tx = coreProgram.transaction.setOwner({
        accounts: {
          owner,
          newOwner: newOwner.publicKey,
          factoryState,
        }
      });
      tx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash

      await expect(connection.sendTransaction(tx, [notOwner])).to.be.rejectedWith(Error)
    })

    it('fails if correct signer but incorrect owner field', async () => {
      await expect(coreProgram.rpc.setOwner({
        accounts: {
          owner: notOwner.publicKey,
          newOwner: newOwner.publicKey,
          factoryState,
        }
      })).to.be.rejectedWith(Error)
    })

    // Test for event and updated owner value
    it('updates owner and emits an event', async function () {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("OwnerChanged", (event, slot) => {
          assert((event.oldOwner as web3.PublicKey).equals(owner))
          assert((event.newOwner as web3.PublicKey).equals(newOwner.publicKey))

          resolve([event, slot]);
        });

        coreProgram.rpc.setOwner({
          accounts: {
            owner,
            newOwner: newOwner.publicKey,
            factoryState,
          }
        });
      });
      await coreProgram.removeEventListener(listener);

      const factoryStateData = await coreProgram.account.factoryState.fetch(factoryState)
      assert(factoryStateData.owner.equals(newOwner.publicKey))
    })

    it('reverts to original owner when signed by the new owner', async () => {
      await coreProgram.rpc.setOwner({
        accounts: {
          owner: newOwner.publicKey,
          newOwner: owner,
          factoryState,
        }, signers: [newOwner]
      });
      const factoryStateData = await coreProgram.account.factoryState.fetch(factoryState)
      assert(factoryStateData.owner.equals(owner))
    })
  })

  describe('#enable_fee_amount', () => {
    it('fails if PDA seeds do not match', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump, fee + 1, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if PDA bump does not match', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump + 1, fee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if caller is not owner', async () => {
      const tx = coreProgram.transaction.enableFeeAmount(feeStateBump, fee, tickSpacing, {
        accounts: {
          owner: notOwner.publicKey,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }, signers: [notOwner]
      })
      tx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash

      await expect(connection.sendTransaction(tx, [notOwner])).to.be.rejectedWith(Error)
    })

    it('fails if fee is too great', async () => {
      const highFee = 1_000_000
      const [highFeeState, highFeeStateBump] = await PublicKey.findProgramAddress(
        [FEE_SEED, u32ToSeed(highFee)],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.enableFeeAmount(highFeeStateBump, highFee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState: highFeeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if tick spacing is too small', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump, fee, 0, {
        accounts: {
          owner,
          factoryState,
          feeState: feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if tick spacing is too large', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump, fee, 16384, {
        accounts: {
          owner,
          factoryState,
          feeState: feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('sets the fee amount and emits an event', async () => {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("FeeAmountEnabled", (event, slot) => {
          assert.equal(event.fee, fee)
          assert.equal(event.tickSpacing, tickSpacing)

          resolve([event, slot]);
        });

        coreProgram.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing, {
          accounts: {
            owner,
            factoryState,
            feeState,
            systemProgram: SystemProgram.programId,
          }
        })
      });
      await coreProgram.removeEventListener(listener);

      const feeStateData = await coreProgram.account.feeState.fetch(feeState)
      assert.equal(feeStateData.bump, feeStateBump)
      assert.equal(feeStateData.fee, fee)
      assert.equal(feeStateData.tickSpacing, tickSpacing)
    })

    it('fails if already initialized', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('cannot change spacing of a fee tier', async () => {
      await expect(coreProgram.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing + 1, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })
  })

  describe('#create_and_init_pool', () => {
    it('derive first observation slot address', async () => {
      [initialObservationState, initialObservationBump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(0)
        ],
        coreProgram.programId
      )
    })

    it('fails if tokens are passed in reverse', async () => {
      // Unlike Uniswap, we must pass the tokens by address sort order
      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
        accounts: {
          poolCreator: owner,
          token0: token1.publicKey,
          token1: token0.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0: vault1,
          vault1: vault0,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if token0 == token1', async () => {
      // Unlike Uniswap, we must pass the tokens by address sort order
      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token0.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1: vault0,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if fee amount is not enabled', async () => {
      const [uninitializedFeeState, _] = await PublicKey.findProgramAddress(
        [FEE_SEED, u32ToSeed(fee + 1)],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token0.publicKey,
          feeState: uninitializedFeeState,
          poolState,
          initialObservationState,
          vault0,
          vault1: vault0,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if starting price is too low', async () => {
      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, new BN(1), {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token1.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith('R')

      await expect(coreProgram.rpc.createAndInitPool(
        poolStateBump,
        initialObservationBump,
        MIN_SQRT_RATIO.subn(1), {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token1.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith('R')

    })

    it('fails if starting price is too high', async () => {
      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, MAX_SQRT_RATIO, {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token1.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith('R')

      await expect(coreProgram.rpc.createAndInitPool(
        poolStateBump,
        initialObservationBump,
        new BN(2).pow(new BN(64)).subn(1), { // u64::MAX
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token1.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith('R')
    })

    it('creates a new pool and initializes it with a starting price', async () => {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("PoolCreatedAndInitialized", (event, slot) => {
          assert((event.token0 as web3.PublicKey).equals(token0.publicKey))
          assert((event.token1 as web3.PublicKey).equals(token1.publicKey))
          assert.equal(event.fee, fee)
          assert.equal(event.tickSpacing, tickSpacing)
          assert((event.poolState as web3.PublicKey).equals(poolState))
          assert((event.sqrtPriceX32 as BN).eq(initialPriceX32))
          assert.equal(event.tick, initialTick)

          resolve([event, slot]);
        });

        coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
          accounts: {
            poolCreator: owner,
            token0: token0.publicKey,
            token1: token1.publicKey,
            feeState,
            poolState,
            initialObservationState,
            vault0,
            vault1,
            systemProgram: SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
          }
        })
      })
      await coreProgram.removeEventListener(listener)

      // pool state variables
      const poolStateData = await coreProgram.account.poolState.fetch(poolState)
      assert.equal(poolStateData.bump, poolStateBump)
      assert((poolStateData.token0).equals(token0.publicKey))
      assert((poolStateData.token1).equals(token1.publicKey))
      assert.equal(poolStateData.fee, fee)
      assert.equal(poolStateData.tickSpacing, tickSpacing)
      assert(poolStateData.liquidity.eqn(0))
      assert((poolStateData.sqrtPriceX32).eq(initialPriceX32))
      assert.equal(poolStateData.tick, initialTick)
      assert.equal(poolStateData.observationIndex, 0)
      assert.equal(poolStateData.observationCardinality, 1)
      assert.equal(poolStateData.observationCardinalityNext, 1)
      assert(poolStateData.feeGrowthGlobal0X32.eq(new BN(0)))
      assert(poolStateData.feeGrowthGlobal1X32.eq(new BN(0)))
      assert.equal(poolStateData.feeProtocol, 0)
      assert(poolStateData.protocolFeesToken0.eq(new BN(0)))
      assert(poolStateData.protocolFeesToken1.eq(new BN(0)))
      assert(poolStateData.unlocked)

      // first observations slot
      const observationStateData = await coreProgram.account.observationState.fetch(initialObservationState)
      assert.equal(observationStateData.bump, initialObservationBump)
      assert.equal(observationStateData.index, 0)
      assert(observationStateData.tickCumulative.eq(new BN(0)))
      assert(observationStateData.secondsPerLiquidityCumulativeX32.eq(new BN(0)))
      assert(observationStateData.initialized)
      assert.approximately(observationStateData.blockTimestamp, Math.floor(Date.now() / 1000), 60)

      console.log('got pool address', poolState.toString())
    })

    it('fails if already initialized', async () => {
      await expect(coreProgram.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
        accounts: {
          poolCreator: owner,
          token0: token0.publicKey,
          token1: token1.publicKey,
          feeState,
          poolState,
          initialObservationState,
          vault0,
          vault1,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }
      })).to.be.rejectedWith(Error)
    })
  })

  describe('#increase_observation_cardinality_next', () => {
    it('fails if bump does not produce a PDA with observation state seeds', async () => {
      const [observationState, _] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([0]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: observationState,
          isSigner: true,
          isWritable: true
        }]
      })).to.be.rejectedWith('Signature verification failed')

    })

    it('fails if bump is valid but account does not match expected address for current cardinality_next', async () => {
      const [_, observationStateBump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        coreProgram.programId
      )
      const fakeAccount = new Keypair()

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationStateBump]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: fakeAccount.publicKey,
          isSigner: true,
          isWritable: true
        }], signers: [fakeAccount]
      })).to.be.rejectedWith('OS')
    })

    it('fails if a single address is passed with index greater than cardinality_next', async () => {
      const [observationState2, observationState2Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationState2Bump]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: observationState2,
          isSigner: false,
          isWritable: true
        }]
      })).to.be.rejectedWith(/OS|Provided seeds do not result in a valid address/)
    })

    it('increase cardinality by one', async () => {
      const [observationState1, observationState1Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        coreProgram.programId
      )

      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("IncreaseObservationCardinalityNext", (event, slot) => {
          assert.equal(event.observationCardinalityNextOld, 1)
          assert.equal(event.observationCardinalityNextNew, 2)
          resolve([event, slot]);
        });

        coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationState1Bump]), {
          accounts: {
            payer: owner,
            poolState,
            systemProgram: SystemProgram.programId,
          }, remainingAccounts: [{
            pubkey: observationState1,
            isSigner: false,
            isWritable: true
          }]
        })
      })
      await coreProgram.removeEventListener(listener)

      const observationState1Data = await coreProgram.account.observationState.fetch(observationState1)
      console.log('Observation state 1 data', observationState1Data)
      assert.equal(observationState1Data.bump, observationState1Bump)
      assert.equal(observationState1Data.index, 1)
      assert.equal(observationState1Data.blockTimestamp, 1)
      assert(observationState1Data.tickCumulative.eq(new BN(0)))
      assert(observationState1Data.secondsPerLiquidityCumulativeX32.eq(new BN(0)))
      assert.isFalse(observationState1Data.initialized)

      const poolStateData = await coreProgram.account.poolState.fetch(poolState)
      assert.equal(poolStateData.observationCardinalityNext, 2)
    })

    it('fails if accounts are not in ascending order of index', async () => {
      const [observationState2, observationState2Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        coreProgram.programId
      )
      const [observationState3, observationState3Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(3)
        ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationState3Bump, observationState2Bump]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: observationState3,
          isSigner: false,
          isWritable: true
        },
        {
          pubkey: observationState2,
          isSigner: false,
          isWritable: true
        }]
      })).to.be.rejectedWith(/OS|Provided seeds do not result in a valid address/)
    })

    it('fails if a stray account is present between the array of observation accounts', async () => {
      const [observationState2, observationState2Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        coreProgram.programId
      )
      const [observationState3, observationState3Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(3)
        ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationState2Bump, observationState3Bump]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: observationState2,
          isSigner: false,
          isWritable: true
        },
        {
          pubkey: new Keypair().publicKey,
          isSigner: false,
          isWritable: true
        },
        {
          pubkey: observationState3,
          isSigner: false,
          isWritable: true
        }]
      })).to.be.rejectedWith(/OS|Provided seeds do not result in a valid address/)
    })

    it('fails if less than current value of cardinality_next', async () => {
      const [observationState1, observationState1Bump] = await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from([observationState1Bump]), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: [{
          pubkey: observationState1,
          isSigner: false,
          isWritable: true
        }]
      })).to.be.rejectedWith(/OS|Provided seeds do not result in a valid address/)
    })

    const MAX_OBSERVATION_INITS_PER_IX = 20

    it('fails if compute unit limit reached by passing more accounts than max limit', async () => {
      const bumps: number[] = []
      const observationAccounts: {
        pubkey: anchor.web3.PublicKey;
        isSigner: boolean;
        isWritable: boolean;
      }[] = []

      // max limit is approximate. Add a larger delta so that tests always pass
      for (let i = 2; i < 2 + MAX_OBSERVATION_INITS_PER_IX + 5; i++) {
        const [observationState, observationStateBump] = await PublicKey.findProgramAddress(
          [
            OBSERVATION_SEED,
            token0.publicKey.toBuffer(),
            token1.publicKey.toBuffer(),
            u32ToSeed(fee),
            u16ToSeed(i)
          ],
          coreProgram.programId
        )
        bumps.push(observationStateBump)
        observationAccounts.push({
          pubkey: observationState,
          isSigner: false,
          isWritable: true
        })
      }

      await expect(coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from(bumps), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: observationAccounts
      })).to.be.rejectedWith(Error)
    })

    it('increase cardinality by max possible amount per instruction permitted by compute budget', async () => {
      const bumps: number[] = []
      const observationAccounts: {
        pubkey: anchor.web3.PublicKey;
        isSigner: boolean;
        isWritable: boolean;
      }[] = []
      const currentCardinality = 2

      for (let i = 0; i < MAX_OBSERVATION_INITS_PER_IX; i++) {
        const [observationState, observationStateBump] = await PublicKey.findProgramAddress(
          [
            OBSERVATION_SEED,
            token0.publicKey.toBuffer(),
            token1.publicKey.toBuffer(),
            u32ToSeed(fee),
            u16ToSeed(currentCardinality + i)
          ],
          coreProgram.programId
        )
        bumps.push(observationStateBump)
        observationAccounts.push({
          pubkey: observationState,
          isSigner: false,
          isWritable: true
        })
      }

      await coreProgram.rpc.increaseObservationCardinalityNext(Buffer.from(bumps), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: observationAccounts
      })

      const poolStateData = await coreProgram.account.poolState.fetch(poolState)
      assert.equal(poolStateData.observationCardinalityNext, currentCardinality + MAX_OBSERVATION_INITS_PER_IX)

      for (let i = 0; i < MAX_OBSERVATION_INITS_PER_IX; i++) {
        const observationAccount = observationAccounts[i].pubkey
        const observationStateData = await coreProgram.account.observationState.fetch(observationAccount)
        assert.equal(observationStateData.bump, bumps[i])
        assert.equal(observationStateData.index, currentCardinality + i)
        assert.equal(observationStateData.blockTimestamp, 1)
        assert(observationStateData.tickCumulative.eq(new BN(0)))
        assert(observationStateData.secondsPerLiquidityCumulativeX32.eq(new BN(0)))
        assert.isFalse(observationStateData.initialized)
      }
    })
  })

  describe('#set_fee_protocol', () => {
    it('cannot be changed by addresses that are not owner', async () => {
      await expect(coreProgram.rpc.setFeeProtocol(6, 6, {
        accounts: {
          owner: notOwner.publicKey,
          poolState,
          factoryState,
        }, signers: [notOwner]
      })).to.be.rejectedWith(Error)
    })

    it('cannot be changed out of bounds', async () => {
      await expect(coreProgram.rpc.setFeeProtocol(3, 3, {
        accounts: {
          owner,
          poolState,
          factoryState,
        }
      })).to.be.rejectedWith(Error)

      await expect(coreProgram.rpc.setFeeProtocol(11, 11, {
        accounts: {
          owner,
          poolState,
          factoryState,
        }
      })).to.be.rejectedWith(Error)
    })

    it('can be changed by owner', async () => {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("SetFeeProtocolEvent", (event, slot) => {
          assert((event.poolState as web3.PublicKey).equals(poolState))
          assert.equal(event.feeProtocol0Old, 0)
          assert.equal(event.feeProtocol1Old, 0)
          assert.equal(event.feeProtocol0, 6)
          assert.equal(event.feeProtocol1, 6)

          resolve([event, slot]);
        });

        coreProgram.rpc.setFeeProtocol(6, 6, {
          accounts: {
            owner,
            poolState,
            factoryState,
          }
        })
      })
      await coreProgram.removeEventListener(listener)

      const poolStateData = await coreProgram.account.poolState.fetch(poolState)
      assert.equal((6 << 4) + 6, 102)
      assert.equal(poolStateData.feeProtocol, 102)
    })
  })

  describe('#collect_protocol', () => {
    it('creates token accounts for recipient', async () => {
      feeRecipientWallet0 = await token0.createAssociatedTokenAccount(protocolFeeRecipient.publicKey)
      feeRecipientWallet1 = await token1.createAssociatedTokenAccount(protocolFeeRecipient.publicKey)
    })

    it('fails if caller is not owner', async () => {
      await expect(coreProgram.rpc.collectProtocol(MaxU64, MaxU64, {
        accounts: {
          owner: notOwner,
          factoryState,
          poolState,
          vault0,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if vault 0 address is not valid', async () => {
      await expect(coreProgram.rpc.collectProtocol(MaxU64, MaxU64, {
        accounts: {
          owner: notOwner,
          factoryState,
          poolState,
          vault0: new Keypair().publicKey,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if vault 1 address is not valid', async () => {
      await expect(coreProgram.rpc.collectProtocol(MaxU64, MaxU64, {
        accounts: {
          owner: notOwner,
          factoryState,
          poolState,
          vault0,
          vault1: new Keypair().publicKey,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })).to.be.rejectedWith(Error)
    })

    it('no token transfers if no fees', async () => {
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("CollectProtocolEvent", (event, slot) => {
          assert((event.poolState as web3.PublicKey).equals(poolState))
          assert((event.sender as web3.PublicKey).equals(owner))
          assert((event.amount0 as BN).eqn(0))
          assert((event.amount1 as BN).eqn(0))

          resolve([event, slot]);
        });

        coreProgram.rpc.collectProtocol(MaxU64, MaxU64, {
          accounts: {
            owner,
            factoryState,
            poolState,
            vault0,
            vault1,
            recipientWallet0: feeRecipientWallet0,
            recipientWallet1: feeRecipientWallet1,
            tokenProgram: TOKEN_PROGRAM_ID,
          }
        })
      })
      await coreProgram.removeEventListener(listener)

      const poolStateData = await coreProgram.account.poolState.fetch(poolState)
      assert(poolStateData.protocolFeesToken0.eqn(0))
      assert(poolStateData.protocolFeesToken1.eqn(0))

      const recipientWallet0Info = await token0.getAccountInfo(feeRecipientWallet0)
      const recipientWallet1Info = await token1.getAccountInfo(feeRecipientWallet1)
      assert(recipientWallet0Info.amount.eqn(0))
      assert(recipientWallet1Info.amount.eqn(0))
    })

    // TODO remaining tests after swap component is ready

  })

  it('setup position manager accounts', async () => {
    [tickLowerState, tickLowerStateBump] = await PublicKey.findProgramAddress([
      TICK_SEED,
      token0.publicKey.toBuffer(),
      token1.publicKey.toBuffer(),
      u32ToSeed(fee),
      u32ToSeed(tickLower)
    ],
      coreProgram.programId
    );

    [tickUpperState, tickUpperStateBump] = await PublicKey.findProgramAddress([
      TICK_SEED,
      token0.publicKey.toBuffer(),
      token1.publicKey.toBuffer(),
      u32ToSeed(fee),
      u32ToSeed(tickUpper)
    ],
      coreProgram.programId
    );

    [bitmapLower, bitmapLowerBump] = await PublicKey.findProgramAddress([
      BITMAP_SEED,
      token0.publicKey.toBuffer(),
      token1.publicKey.toBuffer(),
      u32ToSeed(fee),
      u16ToSeed(wordPosLower),
    ],
      coreProgram.programId
    );
    [bitmapUpper, bitmapUpperBump] = await PublicKey.findProgramAddress([
      BITMAP_SEED,
      token0.publicKey.toBuffer(),
      token1.publicKey.toBuffer(),
      u32ToSeed(fee),
      u16ToSeed(wordPosUpper),
    ],
      coreProgram.programId
    );

    [corePositionState, corePositionBump] = await PublicKey.findProgramAddress([
      POSITION_SEED,
      token0.publicKey.toBuffer(),
      token1.publicKey.toBuffer(),
      u32ToSeed(fee),
      // posMgrState.toBuffer(),
      factoryState.toBuffer(),
      u32ToSeed(tickLower),
      u32ToSeed(tickUpper)
    ],
      coreProgram.programId
    );


    positionNftAccount = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      nftMintKeypair.publicKey,
      owner,
    )

    const nftMint = new Token(
      connection,
      nftMintKeypair.publicKey,
      TOKEN_PROGRAM_ID,
      mintAuthority
    )

    metadataAccount = (
      await web3.PublicKey.findProgramAddress(
        [
          Buffer.from('metadata'),
          metaplex.programs.metadata.MetadataProgram.PUBKEY.toBuffer(),
          nftMintKeypair.publicKey.toBuffer(),
        ],
        metaplex.programs.metadata.MetadataProgram.PUBKEY,
      )
    )[0];

    [tokenizedPositionState, tokenizedPositionBump] = await PublicKey.findProgramAddress([
      POSITION_SEED,
      nftMintKeypair.publicKey.toBuffer()
    ],
      coreProgram.programId
      // mgrProgram.programId
    );
  })

  describe('#init_tick_account', () => {
    it('fails if tick is lower than limit', async () => {
      const [invalidLowTickState, invalidLowTickBump] = await PublicKey.findProgramAddress([
        TICK_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        u32ToSeed(MIN_TICK - 1)
      ],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.initTickAccount(invalidLowTickBump, MIN_TICK - 1, {
        accounts: {
          signer: owner,
          poolState,
          tickState: invalidLowTickState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TLM')
    })

    it('fails if tick is higher than limit', async () => {
      const [invalidUpperTickState, invalidUpperTickBump] = await PublicKey.findProgramAddress([
        TICK_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        u32ToSeed(MAX_TICK + 1)
      ],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.initTickAccount(invalidUpperTickBump, MAX_TICK + 1, {
        accounts: {
          signer: owner,
          poolState,
          tickState: invalidUpperTickState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TUM')
    })

    it('fails if tick is not a multiple of tick spacing', async () => {
      const invalidTick = 5
      const [tickState, tickBump] = await PublicKey.findProgramAddress([
        TICK_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        u32ToSeed(invalidTick)
      ],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.initTickAccount(tickBump, invalidTick, {
        accounts: {
          signer: owner,
          poolState,
          tickState: tickState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TMS')
    })

    it('creates new tick accounts for lower and upper ticks', async () => {
      await coreProgram.rpc.initTickAccount(tickLowerStateBump, tickLower, {
        accounts: {
          signer: owner,
          poolState,
          tickState: tickLowerState,
          systemProgram: SystemProgram.programId,
        }
      })

      await coreProgram.rpc.initTickAccount(tickUpperStateBump, tickUpper, {
        accounts: {
          signer: owner,
          poolState,
          tickState: tickUpperState,
          systemProgram: SystemProgram.programId,
        }
      })

      const tickStateLowerData = await coreProgram.account.tickState.fetch(tickLowerState)
      assert.equal(tickStateLowerData.bump, tickLowerStateBump)
      assert.equal(tickStateLowerData.tick, tickLower)

      const tickStateUpperData = await coreProgram.account.tickState.fetch(tickUpperState)
      assert.equal(tickStateUpperData.bump, tickUpperStateBump)
      assert.equal(tickStateUpperData.tick, tickUpper)
    })
  })

  describe('#init_bitmap_account', () => {
    const minWordPos = (MIN_TICK / tickSpacing) >> 8
    const maxWordPos = (MAX_TICK / tickSpacing) >> 8

    it('fails if tick is lower than limit', async () => {
      const [invalidBitmapLower, invalidBitmapLowerBump] = await PublicKey.findProgramAddress([
        BITMAP_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        u16ToSeed(minWordPos - 1),
      ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.initBitmapAccount(invalidBitmapLowerBump, minWordPos - 1, {
        accounts: {
          signer: owner,
          poolState,
          bitmapState: invalidBitmapLower,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TLM')
    })

    it('fails if tick is higher than limit', async () => {
      const [invalidBitmapUpper, invalidBitmapUpperBump] = await PublicKey.findProgramAddress([
        BITMAP_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        u16ToSeed(maxWordPos + 1),
      ],
        coreProgram.programId
      )

      await expect(coreProgram.rpc.initBitmapAccount(invalidBitmapUpperBump, maxWordPos + 1, {
        accounts: {
          signer: owner,
          poolState,
          bitmapState: invalidBitmapUpper,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TUM')
    })

    it('creates new bitmap account for lower and upper ticks', async () => {
      await coreProgram.rpc.initBitmapAccount(bitmapLowerBump, wordPosLower, {
        accounts: {
          signer: owner,
          poolState,
          bitmapState: bitmapLower,
          systemProgram: SystemProgram.programId,
        }
      })

      const bitmapLowerData = await coreProgram.account.tickBitmapState.fetch(bitmapLower)
      assert.equal(bitmapLowerData.bump, bitmapLowerBump)
      assert.equal(bitmapLowerData.wordPos, wordPosLower)

      // bitmap upper = bitmap lower
    })
  })

  describe('#init_position_account', () => {
    it('fails if tick lower is not less than tick upper', async () => {
      const [invalidPosition, invalidPositionBump] = await PublicKey.findProgramAddress([
        POSITION_SEED,
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee),
        factoryState.toBuffer(),
        // posMgrState.toBuffer(),
        u32ToSeed(tickUpper), // upper first
        u32ToSeed(tickLower),
      ],
        coreProgram.programId
      );

      await expect(coreProgram.rpc.initPositionAccount(invalidPositionBump, {
        accounts: {
          signer: owner,
          // recipient: posMgrState,
          recipient: factoryState,
          poolState,
          tickLowerState: tickUpperState,
          tickUpperState: tickLowerState,
          positionState: invalidPosition,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith('TLU')
    })

    it('creates a new position account', async () => {
      await coreProgram.rpc.initPositionAccount(corePositionBump, {
        accounts: {
          signer: owner,
          // recipient: posMgrState,
          recipient: factoryState,
          poolState,
          tickLowerState,
          tickUpperState,
          positionState: corePositionState,
          systemProgram: SystemProgram.programId,
        }
      })

      const corePositionData = await coreProgram.account.positionState.fetch(corePositionState)
      assert.equal(corePositionData.bump, corePositionBump)
    })
  })

  describe('#mint_tokenized_position', () => {

    it('generate observation PDAs', async () => {
      const {
        observationIndex,
        observationCardinalityNext
      } = await coreProgram.account.poolState.fetch(poolState)

      latestObservationState = (await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(observationIndex)
        ],
        coreProgram.programId
      ))[0]

      nextObservationState = (await PublicKey.findProgramAddress(
        [
          OBSERVATION_SEED,
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed((observationIndex + 1) % observationCardinalityNext)
        ],
        coreProgram.programId
      ))[0]
    })

    it('fails if past deadline', async () => {
      // connection.slot
      const deadline = new BN(Date.now() / 1000 - 10_000)

      await expect(coreProgram.rpc.mintTokenizedPosition(tokenizedPositionBump,
        amount0Desired,
        amount1Desired,
        amount0Minimum,
        amount1Minimum,
        deadline, {
        accounts: {
          minter: owner,
          recipient: owner,
          factoryState,
          nftMint: nftMintKeypair.publicKey,
          nftAccount: positionNftAccount,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          tokenAccount0: minterWallet0,
          tokenAccount1: minterWallet1,
          vault0,
          vault1,
          latestObservationState,
          nextObservationState,
          tokenizedPositionState,
          coreProgram: coreProgram.programId,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
        }, signers: [nftMintKeypair],
      })).to.be.rejectedWith('Transaction too old')
    })

    it('mint tokenized', async () => {
      const deadline = new BN(Date.now() / 1000 + 10_000)

      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("IncreaseLiquidityEvent", (event, slot) => {
          assert((event.tokenId as web3.PublicKey).equals(nftMintKeypair.publicKey))
          assert((event.amount0 as BN).eqn(0))
          assert((event.amount1 as BN).eq(amount1Desired))

          resolve([event, slot]);
        });

        coreProgram.rpc.mintTokenizedPosition(tokenizedPositionBump,
          amount0Desired,
          amount1Desired,
          amount0Minimum,
          amount1Minimum,
          deadline, {
          accounts: {
            minter: owner,
            recipient: owner,
            factoryState,
            nftMint: nftMintKeypair.publicKey,
            nftAccount: positionNftAccount,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            tokenAccount0: minterWallet0,
            tokenAccount1: minterWallet1,
            vault0,
            vault1,
            latestObservationState,
            nextObservationState,
            tokenizedPositionState,
            coreProgram: coreProgram.programId,
            systemProgram: SystemProgram.programId,
            rent: web3.SYSVAR_RENT_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID
          }, signers: [nftMintKeypair],
        })
      })
      await coreProgram.removeEventListener(listener)

      const nftMint = new Token(
        connection,
        nftMintKeypair.publicKey,
        TOKEN_PROGRAM_ID,
        new Keypair()
      )
      const nftMintInfo = await nftMint.getMintInfo()
      assert.equal(nftMintInfo.decimals, 0)
      const nftAccountInfo = await nftMint.getAccountInfo(positionNftAccount)
      console.log('NFT account info', nftAccountInfo)
      assert(nftAccountInfo.amount.eqn(1))

      const tokenizedPositionData = await coreProgram.account.tokenizedPositionState.fetch(tokenizedPositionState)
      console.log('Tokenized position', tokenizedPositionData)
      console.log('liquidity inside position', tokenizedPositionData.liquidity.toNumber())
      assert.equal(tokenizedPositionData.bump, tokenizedPositionBump)
      assert(tokenizedPositionData.poolId.equals(poolState))
      assert(tokenizedPositionData.mint.equals(nftMintKeypair.publicKey))
      assert.equal(tokenizedPositionData.tickLower, tickLower)
      assert.equal(tokenizedPositionData.tickUpper, tickUpper)
      assert(tokenizedPositionData.feeGrowthInside0LastX32.eqn(0))
      assert(tokenizedPositionData.feeGrowthInside1LastX32.eqn(0))
      assert(tokenizedPositionData.tokensOwed0.eqn(0))
      assert(tokenizedPositionData.tokensOwed1.eqn(0))

      const vault0State = await token0.getAccountInfo(vault0)
      assert(vault0State.amount.eqn(0))
      const vault1State = await token1.getAccountInfo(vault1)
      assert(vault1State.amount.eqn(1_000_000))

      const tickLowerData = await coreProgram.account.tickState.fetch(tickLowerState)
      console.log('Tick lower', tickLowerData)
      const tickUpperData = await coreProgram.account.tickState.fetch(tickUpperState)
      console.log('Tick upper', tickUpperData)

      // check if ticks are correctly initialized on the bitmap
      const tickLowerBitmapData = await coreProgram.account.tickBitmapState.fetch(bitmapLower)
      const tickLowerPos = (tickLower / tickSpacing) % 256
      const tickUpperPos = (tickUpper / tickSpacing) % 256
      const expectedBitmap = [3, 2, 1, 0].map(x => {
        let word = new BN(0)
        if (tickLowerPos >= x * 64) {
          const newWord = new BN(1).shln(tickLowerPos - x * 64)
          word = word.add(newWord)
        }
        if (tickUpperPos >= x * 64) {
          word = word.setn(tickUpperPos - x * 64)
          const newWord = new BN(1).shln(tickUpperPos - x * 64)
          word = word.add(newWord)
        }
        return word
      }).reverse()
      for (let i = 0; i < 4; i++) {
        assert(tickLowerBitmapData.word[i].eq(expectedBitmap[i]))
      }

      const corePositionData = await coreProgram.account.positionState.fetch(corePositionState)
      console.log('Core position data', corePositionData)

      // TODO test remaining fields later
      // Look at uniswap tests for reference
    })
  })

  const nftMint = new Token(
    connection,
    nftMintKeypair.publicKey,
    TOKEN_PROGRAM_ID,
    notOwner
  )

  describe('#add_metaplex_metadata', () => {
    it('Add metadata to a generated position', async () => {
      await coreProgram.rpc.addMetaplexMetadata({
        accounts: {
          payer: owner,
          factoryState,
          nftMint: nftMintKeypair.publicKey,
          tokenizedPositionState,
          metadataAccount,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          metadataProgram: metaplex.programs.metadata.MetadataProgram.PUBKEY,
        }
      })

      const nftMintInfo = await nftMint.getMintInfo()
      assert.isNull(nftMintInfo.mintAuthority)
      const metadata = await Metadata.load(connection, metadataAccount)
      assert.equal(metadata.data.mint, nftMint.publicKey.toString())
      assert.equal(metadata.data.updateAuthority, factoryState.toString())
      assert.equal(metadata.data.data.name, 'Cyclos Positions NFT-V1')
      assert.equal(metadata.data.data.symbol, 'CYS-POS')
      assert.equal(metadata.data.data.uri, 'https://api.cyclos.io/mint=' + nftMint.publicKey.toString())
      assert.deepEqual(metadata.data.data.creators, [{
        address: factoryState.toString(),
        // @ts-ignore
        verified: 1,
        share: 100,
      }])
      assert.equal(metadata.data.data.sellerFeeBasisPoints, 0)
      // @ts-ignore
      assert.equal(metadata.data.isMutable, 0)
    })

    it('fails if metadata is already set', async () => {
      await expect(coreProgram.rpc.addMetaplexMetadata({
        accounts: {
          payer: owner,
          factoryState,
          nftMint: nftMintKeypair.publicKey,
          tokenizedPositionState,
          metadataAccount,
          systemProgram: SystemProgram.programId,
          rent: web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
          metadataProgram: metaplex.programs.metadata.MetadataProgram.PUBKEY,
        }
      })).to.be.rejectedWith(Error)
    })
  })

  describe('#increase_liquidity', () => {
    it('fails if past deadline', async () => {
      const deadline = new BN(Date.now() / 1000 - 100_000)
      await expect(coreProgram.rpc.increaseLiquidity(
        amount0Desired,
        amount1Desired,
        amount0Minimum,
        amount1Minimum,
        deadline, {
        accounts: {
          payer: owner,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          tokenAccount0: minterWallet0,
          tokenAccount1: minterWallet1,
          vault0,
          vault1,
          latestObservationState,
          nextObservationState,
          tokenizedPositionState,
          coreProgram: coreProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }
      )).to.be.rejectedWith('Transaction too old')
    })
    it('Add token 1 to the position', async () => {
      const deadline = new BN(Date.now() / 1000 + 10_000)
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("IncreaseLiquidityEvent", (event, slot) => {
          assert((event.tokenId as web3.PublicKey).equals(nftMintKeypair.publicKey))
          assert((event.amount0 as BN).eqn(0))
          assert((event.amount1 as BN).eq(amount1Desired))

          resolve([event, slot]);
        });

        coreProgram.rpc.increaseLiquidity(
          amount0Desired,
          amount1Desired,
          amount0Minimum,
          amount1Minimum,
          deadline, {
          accounts: {
            payer: owner,
            factoryState,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            tokenAccount0: minterWallet0,
            tokenAccount1: minterWallet1,
            vault0,
            vault1,
            latestObservationState,
            nextObservationState,
            tokenizedPositionState,
            coreProgram: coreProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        }
        )
      })
      await coreProgram.removeEventListener(listener)

      const vault0State = await token0.getAccountInfo(vault0)
      assert(vault0State.amount.eqn(0))
      const vault1State = await token1.getAccountInfo(vault1)
      assert(vault1State.amount.eqn(2_000_000))

      // TODO test remaining fields later
      // Look at uniswap tests for reference
    })

    // To check slippage, we must add liquidity in a price range around
    // current price
  })

  describe('#decrease_liquidity', () => {
    const liquidity = new BN(1999599283)
    const amount1Desired = new BN(999999)

    it('fails if past deadline', async () => {
      const deadline = new BN(Date.now() / 1000 - 100_000)
      await expect(coreProgram.rpc.decreaseLiquidity(
        liquidity,
        new BN(0),
        amount1Desired,
        deadline, {
        accounts: {
          ownerOrDelegate: owner,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )).to.be.rejectedWith('Transaction too old')
    })

    it('fails if not called by the owner', async () => {
      const deadline = new BN(Date.now() / 1000 + 10_000)
      await expect(coreProgram.rpc.decreaseLiquidity(
        liquidity,
        new BN(0),
        amount1Desired,
        deadline, {
        accounts: {
          ownerOrDelegate: notOwner,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )).to.be.rejectedWith(Error)
    })

    it('fails if past slippage tolerance', async () => {
      const deadline = new BN(Date.now() / 1000 + 10_000)
      await expect(coreProgram.rpc.decreaseLiquidity(
        liquidity,
        new BN(0),
        new BN(1_000_000), // 999_999 available
        deadline, {
        accounts: {
          ownerOrDelegate: owner,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )).to.be.rejectedWith('Price slippage check')
    })



    it('generate a temporary NFT account for testing', async () => {
      temporaryNftHolder = await nftMint.createAssociatedTokenAccount(mintAuthority.publicKey)
    })

    it('fails if NFT token account for the user is empty', async () => {
      const transferTx = new web3.Transaction()
      transferTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      transferTx.add(Token.createTransferInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        temporaryNftHolder,
        owner,
        [],
        1
      ))

      await anchor.getProvider().send(transferTx)

      const deadline = new BN(Date.now() / 1000 + 10_000)
      await expect(coreProgram.rpc.decreaseLiquidity(
        liquidity,
        new BN(0),
        amount1Desired,
        deadline, {
        accounts: {
          ownerOrDelegate: owner,
          nftAccount: positionNftAccount, // no balance
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )).to.be.rejectedWith('Not approved')

      // send the NFT back to the original owner
      await nftMint.transfer(
        temporaryNftHolder,
        positionNftAccount,
        mintAuthority,
        [],
        1
      )
    })

    it('burn half of the position liquidity as owner', async () => {
      const deadline = new BN(Date.now() / 1000 + 10_000)
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("DecreaseLiquidityEvent", (event, slot) => {
          assert((event.tokenId as web3.PublicKey).equals(nftMintKeypair.publicKey))
          assert((event.liquidity as BN).eq(liquidity))
          assert((event.amount0 as BN).eqn(0))
          assert((event.amount1 as BN).eq(amount1Desired))

          resolve([event, slot]);
        });

        coreProgram.rpc.decreaseLiquidity(
          liquidity,
          new BN(0),
          amount1Desired,
          deadline, {
          accounts: {
            ownerOrDelegate: owner,
            nftAccount: positionNftAccount,
            tokenizedPositionState,
            factoryState,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            latestObservationState,
            nextObservationState,
            coreProgram: coreProgram.programId
          }
        }
        )
      })
      await coreProgram.removeEventListener(listener)
      const tokenizedPositionData = await coreProgram.account.tokenizedPositionState.fetch(tokenizedPositionState)
      assert(tokenizedPositionData.tokensOwed0.eqn(0))
      assert(tokenizedPositionData.tokensOwed1.eqn(999999))
    })

    it('fails if 0 tokens are delegated', async () => {
      const approveTx = new web3.Transaction()
      approveTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      approveTx.add(Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        mintAuthority.publicKey,
        owner,
        [],
        0
      ))
      await anchor.getProvider().send(approveTx)

      const deadline = new BN(Date.now() / 1000 + 10_000)
      const tx = coreProgram.transaction.decreaseLiquidity(
        new BN(1_000),
        new BN(0),
        new BN(0),
        deadline, {
        accounts: {
          ownerOrDelegate: mintAuthority.publicKey,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )
      await expect(connection.sendTransaction(tx, [mintAuthority])).to.be.rejectedWith(Error)
      // TODO see why errors inside functions are not propagating outside
    })

    it('burn liquidity as the delegated authority', async () => {
      const approveTx = new web3.Transaction()
      approveTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      approveTx.add(Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        mintAuthority.publicKey,
        owner,
        [],
        1
      ))
      await anchor.getProvider().send(approveTx)

      const deadline = new BN(Date.now() / 1000 + 10_000)
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("DecreaseLiquidityEvent", (event, slot) => {
          resolve([event, slot]);
        });

        const tx = coreProgram.transaction.decreaseLiquidity(
          new BN(1_000_000),
          new BN(0),
          new BN(0),
          deadline, {
          accounts: {
            ownerOrDelegate: mintAuthority.publicKey,
            nftAccount: positionNftAccount,
            tokenizedPositionState,
            factoryState,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            latestObservationState,
            nextObservationState,
            coreProgram: coreProgram.programId
          }
        }
        )
        connection.sendTransaction(tx, [mintAuthority])
      })
      await coreProgram.removeEventListener(listener)
    })

    it('fails if delegation is revoked', async () => {
      const revokeTx = new web3.Transaction()
      revokeTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      revokeTx.add(Token.createRevokeInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        owner,
        [],
      ))
      await anchor.getProvider().send(revokeTx)

      const deadline = new BN(Date.now() / 1000 + 10_000)
      const tx = coreProgram.transaction.decreaseLiquidity(
        new BN(1_000_000),
        new BN(0),
        new BN(0),
        deadline, {
        accounts: {
          ownerOrDelegate: mintAuthority.publicKey,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId
        }
      }
      )
      // TODO check for 'Not approved' error
      await expect(connection.sendTransaction(tx, [mintAuthority])).to.be.rejectedWith(Error)
    })
  })

  describe('#collect', () => {
    it('fails if both amounts are set as 0', async () => {
      await expect(coreProgram.rpc.collectFromTokenized(new BN(0), new BN(0), {
        accounts: {
          ownerOrDelegate: owner,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId,
          vault0,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if signer is not the owner or a delegated authority', async () => {
      const tx = coreProgram.transaction.collectFromTokenized(new BN(0), new BN(10), {
        accounts: {
          ownerOrDelegate: notOwner.publicKey,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId,
          vault0,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })
      await expect(connection.sendTransaction(tx, [notOwner])).to.be.rejectedWith(Error)
    })

    it('fails delegated amount is 0', async () => {
      const approveTx = new web3.Transaction()
      approveTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      approveTx.add(Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        mintAuthority.publicKey,
        owner,
        [],
        0
      ))
      await anchor.getProvider().send(approveTx)

      const tx = coreProgram.transaction.collectFromTokenized(new BN(0), new BN(10), {
        accounts: {
          ownerOrDelegate: mintAuthority.publicKey,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId,
          vault0,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })
      await expect(connection.sendTransaction(tx, [mintAuthority])).to.be.rejectedWith(Error)
    })

    it('fails if NFT token account is empty', async () => {
      const transferTx = new web3.Transaction()
      transferTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      transferTx.add(Token.createTransferInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        temporaryNftHolder,
        owner,
        [],
        1
      ))
      await anchor.getProvider().send(transferTx)

      await expect(coreProgram.rpc.collectFromTokenized(new BN(0), new BN(10), {
        accounts: {
          ownerOrDelegate: owner,
          nftAccount: positionNftAccount,
          tokenizedPositionState,
          factoryState,
          poolState,
          corePositionState,
          tickLowerState,
          tickUpperState,
          bitmapLower,
          bitmapUpper,
          latestObservationState,
          nextObservationState,
          coreProgram: coreProgram.programId,
          vault0,
          vault1,
          recipientWallet0: feeRecipientWallet0,
          recipientWallet1: feeRecipientWallet1,
          tokenProgram: TOKEN_PROGRAM_ID,
        }
      })).to.be.rejectedWith('Not approved')

      // send the NFT back to the original owner
      await nftMint.transfer(
        temporaryNftHolder,
        positionNftAccount,
        mintAuthority,
        [],
        1
      )
    })

    it('collect a portion of owed tokens as owner', async () => {
      const amount0Max = new BN(0)
      const amount1Max = new BN(10)
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("CollectTokenizedEvent", (event, slot) => {
          assert((event.tokenId as web3.PublicKey).equals(nftMintKeypair.publicKey))
          assert((event.amount0 as BN).eq(amount0Max))
          assert((event.amount1 as BN).eq(amount1Max))
          assert((event.recipientWallet0 as web3.PublicKey).equals(feeRecipientWallet0))
          assert((event.recipientWallet1 as web3.PublicKey).equals(feeRecipientWallet1))

          resolve([event, slot]);
        });

        coreProgram.rpc.collectFromTokenized(amount0Max, amount1Max, {
          accounts: {
            ownerOrDelegate: owner,
            nftAccount: positionNftAccount,
            tokenizedPositionState,
            factoryState,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            latestObservationState,
            nextObservationState,
            coreProgram: coreProgram.programId,
            vault0,
            vault1,
            recipientWallet0: feeRecipientWallet0,
            recipientWallet1: feeRecipientWallet1,
            tokenProgram: TOKEN_PROGRAM_ID,
          }
        })
      })
      await coreProgram.removeEventListener(listener)

      const corePositionData = await coreProgram.account.positionState.fetch(corePositionState)
      assert(corePositionData.tokensOwed0.eqn(0))
      assert(corePositionData.tokensOwed1.eqn(1000489)) // minus 10

      const tokenizedPositionData = await coreProgram.account.tokenizedPositionState.fetch(tokenizedPositionState)
      assert(tokenizedPositionData.tokensOwed0.eqn(0))
      assert(tokenizedPositionData.tokensOwed1.eqn(1000489))

      const recipientWallet0Info = await token0.getAccountInfo(feeRecipientWallet0)
      const recipientWallet1Info = await token1.getAccountInfo(feeRecipientWallet1)
      assert(recipientWallet0Info.amount.eqn(0))
      assert(recipientWallet1Info.amount.eqn(10))

      const vault0Info = await token0.getAccountInfo(vault0)
      const vault1Info = await token1.getAccountInfo(vault1)
      assert(vault0Info.amount.eqn(0))
      assert(vault1Info.amount.eqn(1999990)) // minus 10
    })

    it('collect a portion of owed tokens as the delegated authority', async () => {
      const approveTx = new web3.Transaction()
      approveTx.recentBlockhash = (await connection.getRecentBlockhash()).blockhash
      approveTx.add(Token.createApproveInstruction(
        TOKEN_PROGRAM_ID,
        positionNftAccount,
        mintAuthority.publicKey,
        owner,
        [],
        1
      ))
      await anchor.getProvider().send(approveTx)

      const amount0Max = new BN(0)
      const amount1Max = new BN(10)
      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = coreProgram.addEventListener("CollectTokenizedEvent", (event, slot) => {
          assert((event.tokenId as web3.PublicKey).equals(nftMintKeypair.publicKey))
          assert((event.amount0 as BN).eq(amount0Max))
          assert((event.amount1 as BN).eq(amount1Max))
          assert((event.recipientWallet0 as web3.PublicKey).equals(feeRecipientWallet0))
          assert((event.recipientWallet1 as web3.PublicKey).equals(feeRecipientWallet1))

          resolve([event, slot]);
        });

        const tx = coreProgram.transaction.collectFromTokenized(new BN(0), new BN(10), {
          accounts: {
            ownerOrDelegate: mintAuthority.publicKey,
            nftAccount: positionNftAccount,
            tokenizedPositionState,
            factoryState,
            poolState,
            corePositionState,
            tickLowerState,
            tickUpperState,
            bitmapLower,
            bitmapUpper,
            latestObservationState,
            nextObservationState,
            coreProgram: coreProgram.programId,
            vault0,
            vault1,
            recipientWallet0: feeRecipientWallet0,
            recipientWallet1: feeRecipientWallet1,
            tokenProgram: TOKEN_PROGRAM_ID,
          }
        })
        connection.sendTransaction(tx, [mintAuthority])
      })
      await coreProgram.removeEventListener(listener)

      const corePositionData = await coreProgram.account.positionState.fetch(corePositionState)
      assert(corePositionData.tokensOwed0.eqn(0))
      assert(corePositionData.tokensOwed1.eqn(1000479))

      const tokenizedPositionData = await coreProgram.account.tokenizedPositionState.fetch(tokenizedPositionState)
      assert(tokenizedPositionData.tokensOwed0.eqn(0))
      assert(tokenizedPositionData.tokensOwed1.eqn(1000479))

      const recipientWallet0Info = await token0.getAccountInfo(feeRecipientWallet0)
      const recipientWallet1Info = await token1.getAccountInfo(feeRecipientWallet1)
      assert(recipientWallet0Info.amount.eqn(0))
      assert(recipientWallet1Info.amount.eqn(20))

      const vault0Info = await token0.getAccountInfo(vault0)
      const vault1Info = await token1.getAccountInfo(vault1)
      assert(vault0Info.amount.eqn(0))
      assert(vault1Info.amount.eqn(1999980))
    })
  })

  //   describe('#exact_input_single', () => {
  //     // before swapping, current tick = 10 and price = 4297115210
  //     // active ticks are 0 and 10
  //     // entire liquidity is in token_1

  //     const deadline = new BN(Date.now() / 1000 + 1_000_000)

  //     it('fails if limit price is greater than current pool price', async () => {
  //       const amountIn = new BN(100_000)
  //       const amountOutMinimum = new BN(0)
  //       const sqrtPriceLimitX32 = new BN(4297115220)

  //       await expect(routerProgram.rpc.exactInputSingle(
  //         deadline,
  //         true,
  //         amountIn,
  //         amountOutMinimum,
  //         sqrtPriceLimitX32,
  //         {
  //           accounts: {
  //             signer: owner,
  //             poolState,
  //             tokenAccount0: minterWallet0,
  //             tokenAccount1: minterWallet1,
  //             vault0,
  //             vault1,
  //             latestObservationState,
  //             nextObservationState,
  //             coreProgram: coreProgram.programId,
  //             tokenProgram: TOKEN_PROGRAM_ID,
  //           }, remainingAccounts: [{
  //             pubkey: bitmapLower,
  //             isSigner: false,
  //             isWritable: true
  //           },
  //           // price moves downwards in zero for one swap
  //           {
  //             pubkey: tickUpperState,
  //             isSigner: false,
  //             isWritable: true
  //           }, {
  //             pubkey: tickLowerState,
  //             isSigner: false,
  //             isWritable: true
  //           }]
  //         }
  //       )).to.be.rejectedWith(Error)
  //     })

  //     it('swap upto a limit price for a zero to one swap', async () => {
  //       const amountIn = new BN(100_000)
  //       const amountOutMinimum = new BN(0)
  //       const sqrtPriceLimitX32 = new BN(4297115200) // current price is 4297115210

  //       await routerProgram.rpc.exactInputSingle(
  //         deadline,
  //         true,
  //         amountIn,
  //         amountOutMinimum,
  //         sqrtPriceLimitX32,
  //         {
  //           accounts: {
  //             signer: owner,
  //             poolState,
  //             tokenAccount0: minterWallet0,
  //             tokenAccount1: minterWallet1,
  //             vault0,
  //             vault1,
  //             latestObservationState,
  //             nextObservationState,
  //             coreProgram: coreProgram.programId,
  //             tokenProgram: TOKEN_PROGRAM_ID,
  //           }, remainingAccounts: [{
  //             pubkey: bitmapLower,
  //             isSigner: false,
  //             isWritable: true
  //           },
  //           // price moves downwards in zero for one swap
  //           {
  //             pubkey: tickUpperState,
  //             isSigner: false,
  //             isWritable: true
  //           }, {
  //             pubkey: tickLowerState,
  //             isSigner: false,
  //             isWritable: true
  //           }]
  //         }
  //       )

  //       let poolStateData = await coreProgram.account.poolState.fetch(poolState)
  //       assert(poolStateData.sqrtPriceX32.eq(sqrtPriceLimitX32))
  //     })

  //     it('performs a zero for one swap without a limit price', async () => {
  //       let poolStateDataBefore = await coreProgram.account.poolState.fetch(poolState)
  //       console.log('pool price', poolStateDataBefore.sqrtPriceX32.toNumber())
  //       console.log('pool tick', poolStateDataBefore.tick)

  //       const {
  //         observationIndex,
  //         observationCardinalityNext
  //       } = await coreProgram.account.poolState.fetch(poolState)

  //       latestObservationState = (await PublicKey.findProgramAddress(
  //         [
  //           OBSERVATION_SEED,
  //           token0.publicKey.toBuffer(),
  //           token1.publicKey.toBuffer(),
  //           u32ToSeed(fee),
  //           u16ToSeed(observationIndex)
  //         ],
  //         coreProgram.programId
  //       ))[0]

  //       nextObservationState = (await PublicKey.findProgramAddress(
  //         [
  //           OBSERVATION_SEED,
  //           token0.publicKey.toBuffer(),
  //           token1.publicKey.toBuffer(),
  //           u32ToSeed(fee),
  //           u16ToSeed((observationIndex + 1) % observationCardinalityNext)
  //         ],
  //         coreProgram.programId
  //       ))[0]

  //       const amountIn = new BN(100_000)
  //       const amountOutMinimum = new BN(0)
  //       const sqrtPriceLimitX32 = new BN(0)
  //       await routerProgram.rpc.exactInputSingle(
  //         deadline,
  //         true,
  //         amountIn,
  //         amountOutMinimum,
  //         sqrtPriceLimitX32,
  //         {
  //           accounts: {
  //             signer: owner,
  //             poolState,
  //             tokenAccount0: minterWallet0,
  //             tokenAccount1: minterWallet1,
  //             vault0,
  //             vault1,
  //             latestObservationState,
  //             nextObservationState,
  //             coreProgram: coreProgram.programId,
  //             tokenProgram: TOKEN_PROGRAM_ID,
  //           }, remainingAccounts: [{
  //             pubkey: bitmapLower,
  //             isSigner: false,
  //             isWritable: true
  //           },
  //           // price moves downwards in zero for one swap
  //           {
  //             pubkey: tickLowerState,
  //             isSigner: false,
  //             isWritable: true
  //           }]
  //         }
  //       )
  //       const poolStateDataAfter = await coreProgram.account.poolState.fetch(poolState)
  //       console.log('pool price after', poolStateDataAfter.sqrtPriceX32.toNumber())
  //       console.log('pool tick after', poolStateDataAfter.tick)
  //     })
  //   })

  // })
})

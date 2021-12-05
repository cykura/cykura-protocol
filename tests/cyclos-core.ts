import * as anchor from '@project-serum/anchor';
import { Program, web3, BN, ProgramError } from '@project-serum/anchor';
import { Token, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { assert, expect } from 'chai';
import * as chai from 'chai'
import chaiAsPromised from 'chai-as-promised'
chai.use(chaiAsPromised)

import { CyclosCore } from '../target/types/cyclos_core';
import { MAX_SQRT_RATIO, MIN_SQRT_RATIO, u16ToSeed, u32ToSeed } from './utils';

const { PublicKey, Keypair, SystemProgram } = anchor.web3;

describe('cyclos-core', async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.CyclosCore as Program<CyclosCore>;
  // const program = anchor.workspace.CyclosCore as Program;
  const { connection, wallet } = anchor.getProvider()
  const owner = anchor.getProvider().wallet.publicKey;
  const notOwner = new Keypair()

  const fee = 500;
  const tickSpacing = 10;

  const [factoryState, factoryStateBump] = await PublicKey.findProgramAddress([], program.programId);
  console.log("Factory", factoryState.toString(), factoryStateBump);

  const [feeState, feeStateBump] = await PublicKey.findProgramAddress(
    [u32ToSeed(fee)],
    program.programId
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

  it('Create token mints', async () => {
    const transferSolTx = new web3.Transaction().add(
      web3.SystemProgram.transfer({
        fromPubkey: owner,
        toPubkey: mintAuthority.publicKey,
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

  it('derive pool address', async () => {
    [poolState, poolStateBump] = await PublicKey.findProgramAddress(
      [
        token0.publicKey.toBuffer(),
        token1.publicKey.toBuffer(),
        u32ToSeed(fee)
      ],
      program.programId
    )
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
        listener = program.addEventListener("OwnerChanged", (event, slot) => {
          assert((event.oldOwner as web3.PublicKey).equals(new PublicKey(0)))
          assert((event.newOwner as web3.PublicKey).equals(owner))

          resolve([event, slot]);
        });

        program.rpc.initFactory(factoryStateBump, {
          accounts: {
            owner,
            factoryState,
            systemProgram: SystemProgram.programId,
          }
        });
      });
      await program.removeEventListener(listener);

      const factoryStateData = await program.account.factoryState.fetch(factoryState)
      assert.equal(factoryStateData.bump, factoryStateBump)
      assert(factoryStateData.owner.equals(owner))
    });

    it('Trying to re-initialize factory fails', async () => {
      await expect(program.rpc.initFactory(factoryStateBump, {
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
      const tx = program.transaction.setOwner({
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
      const tx = program.transaction.setOwner({
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
      await expect(program.rpc.setOwner({
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
        listener = program.addEventListener("OwnerChanged", (event, slot) => {
          assert((event.oldOwner as web3.PublicKey).equals(owner))
          assert((event.newOwner as web3.PublicKey).equals(newOwner.publicKey))

          resolve([event, slot]);
        });

        program.rpc.setOwner({
          accounts: {
            owner,
            newOwner: newOwner.publicKey,
            factoryState,
          }
        });
      });
      await program.removeEventListener(listener);

      const factoryStateData = await program.account.factoryState.fetch(factoryState)
      assert(factoryStateData.owner.equals(newOwner.publicKey))
    })

    it('reverts to original owner when signed by the new owner', async () => {
      await program.rpc.setOwner({
        accounts: {
          owner: newOwner.publicKey,
          newOwner: owner,
          factoryState,
        }, signers: [newOwner]
      });
      const factoryStateData = await program.account.factoryState.fetch(factoryState)
      assert(factoryStateData.owner.equals(owner))
    })
  })

  describe('#enable_fee_amount', () => {
    it('fails if PDA seeds do not match', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump, fee + 1, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if PDA bump does not match', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump + 1, fee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if caller is not owner', async () => {
      const tx = program.transaction.enableFeeAmount(feeStateBump, fee, tickSpacing, {
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
        [u32ToSeed(highFee)],
        program.programId
      );

      await expect(program.rpc.enableFeeAmount(highFeeStateBump, highFee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState: highFeeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if tick spacing is too small', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump, fee, 0, {
        accounts: {
          owner,
          factoryState,
          feeState: feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('fails if tick spacing is too large', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump, fee, 16384, {
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
        listener = program.addEventListener("FeeAmountEnabled", (event, slot) => {
          assert.equal(event.fee, fee)
          assert.equal(event.tickSpacing, tickSpacing)

          resolve([event, slot]);
        });

        program.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing, {
          accounts: {
            owner,
            factoryState,
            feeState,
            systemProgram: SystemProgram.programId,
          }
        })
      });
      await program.removeEventListener(listener);

      const feeStateData = await program.account.feeState.fetch(feeState)
      assert.equal(feeStateData.bump, feeStateBump)
      assert.equal(feeStateData.fee, fee)
      assert.equal(feeStateData.tickSpacing, tickSpacing)
    })

    it('fails if already initialized', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })

    it('cannot change spacing of a fee tier', async () => {
      await expect(program.rpc.enableFeeAmount(feeStateBump, fee, tickSpacing + 1, {
        accounts: {
          owner,
          factoryState,
          feeState,
          systemProgram: SystemProgram.programId,
        }
      })).to.be.rejectedWith(Error)
    })
  })

  const initialPriceX32 = new BN(4297115210)
  const initialTick = 10

  describe('#create_and_init_pool', () => {
    it('derive first observation slot address', async () => {
      [initialObservationState, initialObservationBump] = await PublicKey.findProgramAddress(
        [
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(0)
        ],
        program.programId
      )
    })

    it('fails if tokens are passed in reverse', async () => {
      // Unlike Uniswap, we must pass the tokens by address sort order
      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
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
      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
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
        [u32ToSeed(fee + 1)],
        program.programId
      );

      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
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
      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, new BN(1), {
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

      await expect(program.rpc.createAndInitPool(
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
      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, MAX_SQRT_RATIO, {
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

      await expect(program.rpc.createAndInitPool(
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
        listener = program.addEventListener("PoolCreatedAndInitialized", (event, slot) => {
          assert((event.token0 as web3.PublicKey).equals(token0.publicKey))
          assert((event.token1 as web3.PublicKey).equals(token1.publicKey))
          assert.equal(event.fee, fee)
          assert.equal(event.tickSpacing, tickSpacing)
          assert((event.poolState as web3.PublicKey).equals(poolState))
          assert((event.sqrtPriceX32 as BN).eq(initialPriceX32))
          assert.equal(event.tick, initialTick)

          resolve([event, slot]);
        });

        program.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
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
      await program.removeEventListener(listener)

      // pool state variables
      const poolStateData = await program.account.poolState.fetch(poolState)
      assert.equal(poolStateData.bump, poolStateBump)
      assert((poolStateData.token0).equals(token0.publicKey))
      assert((poolStateData.token1).equals(token1.publicKey))
      assert.equal(poolStateData.fee, fee)
      assert.equal(poolStateData.tickSpacing, tickSpacing)
      assert.equal(poolStateData.liquidity, 0)
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
      const observationStateData = await program.account.observationState.fetch(initialObservationState)
      assert.equal(observationStateData.bump, initialObservationBump)
      assert.equal(observationStateData.index, 0)
      assert(observationStateData.tickCumulative.eq(new BN(0)))
      assert(observationStateData.secondsPerLiquidityCumulativeX32.eq(new BN(0)))
      assert(observationStateData.initialized)
      assert.approximately(observationStateData.blockTimestamp, Math.floor(Date.now() / 1000), 10)
    })

    it('fails if already initialized', async () => {
      await expect(program.rpc.createAndInitPool(poolStateBump, initialObservationBump, initialPriceX32, {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        program.programId
      )

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([0]), {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        program.programId
      )
      const fakeAccount = new Keypair()

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([observationStateBump]), {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        program.programId
      )

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([observationState2Bump]), {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        program.programId
      )

      let listener: number
      let [_event, _slot] = await new Promise((resolve, _reject) => {
        listener = program.addEventListener("IncreaseObservationCardinalityNext", (event, slot) => {
          assert.equal(event.observationCardinalityNextOld, 1)
          assert.equal(event.observationCardinalityNextNew, 2)
          resolve([event, slot]);
        });

        program.rpc.increaseObservationCardinalityNext(Buffer.from([observationState1Bump]), {
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
      await program.removeEventListener(listener)

      const observationState1Data = await program.account.observationState.fetch(observationState1)
      console.log('Observation state 1 data', observationState1Data)
      assert.equal(observationState1Data.bump, observationState1Bump)
      assert.equal(observationState1Data.index, 1)
      assert.equal(observationState1Data.blockTimestamp, 1)
      assert(observationState1Data.tickCumulative.eq(new BN(0)))
      assert(observationState1Data.secondsPerLiquidityCumulativeX32.eq(new BN(0)))
      assert.isFalse(observationState1Data.initialized)

      const poolStateData = await program.account.poolState.fetch(poolState)
      assert.equal(poolStateData.observationCardinalityNext, 2)
    })

    it('fails if accounts are not in ascending order of index', async () => {
      const [observationState2, observationState2Bump] = await PublicKey.findProgramAddress(
        [
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        program.programId
      )
      const [observationState3, observationState3Bump] = await PublicKey.findProgramAddress(
        [
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(3)
        ],
        program.programId
      )

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([observationState3Bump, observationState2Bump]), {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(2)
        ],
        program.programId
      )
      const [observationState3, observationState3Bump] = await PublicKey.findProgramAddress(
        [
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(3)
        ],
        program.programId
      )

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([observationState2Bump, observationState3Bump]), {
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
          token0.publicKey.toBuffer(),
          token1.publicKey.toBuffer(),
          u32ToSeed(fee),
          u16ToSeed(1)
        ],
        program.programId
      )

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from([observationState1Bump]), {
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

      for (let i = 2; i < 2 + MAX_OBSERVATION_INITS_PER_IX + 1; i++) {
        const [observationState, observationStateBump] = await PublicKey.findProgramAddress(
          [
            token0.publicKey.toBuffer(),
            token1.publicKey.toBuffer(),
            u32ToSeed(fee),
            u16ToSeed(i)
          ],
          program.programId
        )
        bumps.push(observationStateBump)
        observationAccounts.push({
          pubkey: observationState,
          isSigner: false,
          isWritable: true
        })
      }

      await expect(program.rpc.increaseObservationCardinalityNext(Buffer.from(bumps), {
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
            token0.publicKey.toBuffer(),
            token1.publicKey.toBuffer(),
            u32ToSeed(fee),
            u16ToSeed(currentCardinality + i)
          ],
          program.programId
        )
        bumps.push(observationStateBump)
        observationAccounts.push({
          pubkey: observationState,
          isSigner: false,
          isWritable: true
        })
      }

      await program.rpc.increaseObservationCardinalityNext(Buffer.from(bumps), {
        accounts: {
          payer: owner,
          poolState,
          systemProgram: SystemProgram.programId,
        }, remainingAccounts: observationAccounts
      })

      const poolStateData = await program.account.poolState.fetch(poolState)
      assert.equal(poolStateData.observationCardinalityNext, currentCardinality + MAX_OBSERVATION_INITS_PER_IX)

      for (let i = 0; i < MAX_OBSERVATION_INITS_PER_IX; i++) {
        const observationAccount = observationAccounts[i].pubkey
        const observationStateData = await program.account.observationState.fetch(observationAccount)
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
      await expect(program.rpc.setFeeProtocol(6, 6, {
        accounts: {
          owner: notOwner.publicKey,
          poolState,
          factoryState,
        }, signers: [notOwner]
      })).to.be.rejectedWith(Error)
    })

    it('cannot be changed out of bounds', async () => {
      await expect(program.rpc.setFeeProtocol(3, 3, {
        accounts: {
          owner,
          poolState,
          factoryState,
        }
      })).to.be.rejectedWith(Error)

      await expect(program.rpc.setFeeProtocol(11, 11, {
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
        listener = program.addEventListener("SetFeeProtocolEvent", (event, slot) => {
          assert((event.poolState as web3.PublicKey).equals(poolState))
          assert.equal(event.feeProtocol0Old, 0)
          assert.equal(event.feeProtocol1Old, 0)
          assert.equal(event.feeProtocol0, 6)
          assert.equal(event.feeProtocol1, 6)

          resolve([event, slot]);
        });

        program.rpc.setFeeProtocol(6, 6, {
          accounts: {
            owner,
            poolState,
            factoryState,
          }
        })
      })
      await program.removeEventListener(listener)

      const poolStateData = await program.account.poolState.fetch(poolState)
      assert.equal((6 << 4) + 6, 102)
      assert.equal(poolStateData.feeProtocol, 102)
    })
  })
});

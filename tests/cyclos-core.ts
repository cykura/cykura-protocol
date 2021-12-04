import * as anchor from '@project-serum/anchor';
import { Program, web3, BN } from '@project-serum/anchor';
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
      let listener = null;
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
      let listener = null;
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
      let listener = null;
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
    it('derive observation address', async () => {
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
      let listener = null;
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
});

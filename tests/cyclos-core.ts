import * as anchor from '@project-serum/anchor';
import { Program, web3, BN } from '@project-serum/anchor';
import { CyclosCore } from '../target/types/cyclos_core';
const { PublicKey } = anchor.web3;

describe('cyclos-core', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.CyclosCore as Program<CyclosCore>;

  let factoryState: web3.PublicKey
  let factoryStateBump: number
  let feeState: web3.PublicKey
  let feeStateBump: number

  const fee = 500;
  const tickSpacing = 10;

  before(async () => {
    [factoryState, factoryStateBump] = await PublicKey.findProgramAddress([], program.programId);
    console.log("Factory", factoryState.toString(), factoryStateBump);

    [feeState, feeStateBump] = await PublicKey.findProgramAddress(
      [numberToBigEndian(fee)],
      program.programId
      );
    console.log("Fee", feeState.toString(), feeStateBump);
  })

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize(factoryStateBump, {
      accounts: {
        owner: anchor.getProvider().wallet.publicKey,
        factoryState,
        systemProgram: anchor.web3.SystemProgram.programId,
      }
    });
    console.log("Your transaction signature", tx);
  });

  it('Enable Fee amount', async () => {
    // Add your test here.
    const tx = await program.rpc.enableFeeAmount(fee, tickSpacing, feeStateBump, {
      accounts: {
        owner: anchor.getProvider().wallet.publicKey,
        factoryState,
        feeState,
        systemProgram: anchor.web3.SystemProgram.programId,
      }
    });
    console.log("Your transaction signature", tx);
  });

  it('Update owner', async () => {
    // TODO use different keypair for new owner
    const newOwner = anchor.getProvider().wallet.publicKey
    const tx = await program.rpc.setOwner({
      accounts: {
        owner: anchor.getProvider().wallet.publicKey,
        factoryState,
        newOwner: newOwner
      }
    });

    // TODO read state and match owner
  })


});

export function numberToBigEndian(num: number) {
  const arr = new ArrayBuffer(4)
  const view = new DataView(arr)
  view.setUint32(0, num, false)

  const bigEndianArray = new Uint8Array(arr)
  return bigEndianArray
}

import * as anchor from '@project-serum/anchor';
import { Program, web3, BN } from '@project-serum/anchor';
import { CyclosProtocolV2 } from '../target/types/cyclos_protocol_v2';
const { PublicKey,  } = anchor.web3;

describe('cyclos-protocol-v2', async () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.CyclosProtocolV2 as Program<CyclosProtocolV2>;

  const [factoryState, factoryStateBump] = await PublicKey.findProgramAddress([], program.programId);
  console.log("Factory", factoryState.toString(), factoryStateBump);
  
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

  const fee = 500;
  const tickSpacing = 10;

  const [feeState, feeStateBump] = await PublicKey.findProgramAddress(
    [numberToBigEndian(fee)], 
    program.programId
    );
  console.log("Fee", feeState.toString(), feeStateBump);

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
});

export function numberToBigEndian(num: number) {
  const arr = new ArrayBuffer(4)
  const view = new DataView(arr)
  view.setUint32(0, num, false)

  const bigEndianArray = new Uint8Array(arr)
  return bigEndianArray
}

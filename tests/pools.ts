import * as anchor from '@project-serum/anchor';
import { Program, web3, BN } from '@project-serum/anchor';
// import { Pool } from '../target/types/cyclos_protocol_vanchor test2';
const { PublicKey,  } = anchor.web3;

describe('Pools', async () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  // const program = anchor.workspace.Pool as Program<Pool>;
  const program = anchor.workspace.Pool;

  const [poolState, poolStateBump] = await PublicKey.findProgramAddress([], program.programId);
  console.log("Pool ", poolState.toString(), poolStateBump);

  const token0 = new PublicKey("BRLsMczKuaR5w9vSubF4j8HwEGGprVAyyVgS4EX7DKEg");
  const token1 = new PublicKey("2wmVCSfPxGPjrnMMn7rchp4uaeoTqN39mXFC2zhPdri9");
  const fees = 10_000;
  
  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize(token0, token1, fees, poolStateBump, {
      accounts: {
        owner: anchor.getProvider().wallet.publicKey,
        poolState,
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

import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { CyclosProtocolV2 } from '../target/types/cyclos_protocol_v2';
const { PublicKey } = anchor.web3;

describe('cyclos-protocol-v2', async () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.CyclosProtocolV2 as Program<CyclosProtocolV2>;

  const [factoryState, factoryStateBump] = await PublicKey.findProgramAddress([], program.programId);

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
});

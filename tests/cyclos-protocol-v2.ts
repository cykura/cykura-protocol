import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { CyclosProtocolV2 } from '../target/types/cyclos_protocol_v2';

describe('cyclos-protocol-v2', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.CyclosProtocolV2 as Program<CyclosProtocolV2>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});

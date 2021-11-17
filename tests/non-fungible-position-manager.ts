import * as anchor from '@project-serum/anchor';
import { Program, web3, BN } from '@project-serum/anchor';
import { NonFungiblePositionManager } from '../target/types/non_fungible_position_manager';

describe('non-fungible-position-manager', () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.NonFungiblePositionManager as Program<NonFungiblePositionManager>;

  it('Callback payment', async () => {

  });

});

import * as anchor from '@project-serum/anchor'
import { web3 } from '@project-serum/anchor'
import { PublicKey, SolanaProvider, TransactionEnvelope } from '@saberhq/solana-contrib'
import { ASSOCIATED_TOKEN_PROGRAM_ID, createMintInstructions, getATAAddress, TOKEN_PROGRAM_ID, u64, getTokenAccount } from '@saberhq/token-utils'
import { Token } from '@solana/spl-token'
import { Keypair, SystemProgram, Transaction, LAMPORTS_PER_SOL } from "@solana/web3.js"
import { findEscrowAddress, findGovernorAddress, findLockerAddress, LockerWrapper, TribecaSDK } from '@tribecahq/tribeca-sdk'
import keypairFile from './keypair.json'
import type { Provider } from "@saberhq/solana-contrib";
import * as SPLToken from "@solana/spl-token";
import { 
  MerkleDistributorErrors, 
  MerkleDistributorSDK,
  MerkleDistributorWrapper,
  PendingDistributor,
  findClaimStatusKey, 
  findDistributorKey,
} from "@saberhq/merkle-distributor";
import { BalanceTree } from './utils/balance-tree';



const MAX_NUM_NODES = new u64(3);
const MAX_TOTAL_CLAIM = new u64(1_000_000_000_000);
const ZERO_BYTES32 = Buffer.alloc(32);

export const createAndSeedDistributor = async (
  sdk: MerkleDistributorSDK,
  maxTotalClaim: u64,
  maxNumNodes: u64,
  root: Buffer
): Promise<{
  mint: PublicKey;
  distributor: PublicKey;
  pendingDistributor: PendingDistributor;
}> => {
  const { provider } = sdk;
  // const mint = await createMint(provider, provider.connection, provider.wallet.publicKey, 6);
  // console.log("CYS: ", mint)

  console.log("Yooohooooooo")

  let cysMint = Keypair.fromSecretKey(
    Uint8Array.from([170, 204, 133, 206, 215, 135, 147, 69, 202, 136, 132, 212, 28, 149, 110, 252, 100, 236, 7, 172, 87, 170, 80, 207, 122, 181, 91, 120, 31, 198, 72, 62, 9, 54, 24, 114, 208, 200, 16, 126, 237, 6, 101, 43, 79, 108, 255, 88, 254, 188, 218, 124, 116, 214, 182, 25, 219, 28, 183, 227, 101, 197, 44, 71])
  ); // cxWg5RTK5AiSbBZh7NRg5btsbSrc8ETLXGf7tk3MUez
  const cysTx = new Transaction();  
  cysTx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: cysMint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await SPLToken.Token.getMinBalanceRentForExemptMint(provider.connection),
      programId: SPLToken.TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      SPLToken.TOKEN_PROGRAM_ID, // program id, always token program id
      cysMint.publicKey, // mint account public key
      6, // decimals
      MINT_AUTHORITY.publicKey, // mint authority (an auth to mint token)
      null // freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );
  cysTx.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash;
  const txhash = await provider.send(cysTx, [cysMint])
  console.log(`txhash: ${txhash}`);

  const mint = cysMint.publicKey;




  const pendingDistributor = await sdk.createDistributor({
    root,
    maxTotalClaim,
    maxNumNodes,
    tokenMint: cysMint.publicKey,
  });
  let txBuildx = pendingDistributor.tx.build();
  txBuildx.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash;
  const str1 = txBuildx.serializeMessage().toString('base64');
  console.log(`https://explorer.solana.com/tx/inspector?message=${encodeURIComponent(str1)}&cluster=custom`) 
  let txSig1 = await provider.send(txBuildx, pendingDistributor.tx.signers)
  console.log(`New Distributor::=>  ${txSig1}`);

  // Seed merkle distributor with tokens
  const ix = SPLToken.Token.createMintToInstruction(
    TOKEN_PROGRAM_ID,
    mint,
    pendingDistributor.distributorATA,
    provider.wallet.publicKey,
    [],
    maxTotalClaim
  );
  const tx = new TransactionEnvelope(provider, [ix]);
  let txBuild = tx.build();
  txBuild.recentBlockhash = (await provider.connection.getLatestBlockhash()).blockhash;
  const str = txBuild.serializeMessage().toString('base64');
  console.log(`https://explorer.solana.com/tx/inspector?message=${encodeURIComponent(str)}&cluster=custom`) 
  let txSig = await provider.send(txBuild, tx.signers)
  console.log(`New Distributor::=>  ${txSig}`);

  return {
    mint,
    distributor: pendingDistributor.distributor,
    pendingDistributor,
  };
};


export async function createMint(
  provider: Provider,
  connection: web3.Connection,
  authority?: PublicKey,
  decimals?: number
): Promise<PublicKey> {
  if (authority === undefined) {
    authority = provider.wallet.publicKey;
  }
  const mint = Keypair.fromSecretKey(
    Uint8Array.from([170, 204, 133, 206, 215, 135, 147, 69, 202, 136, 132, 212, 28, 149, 110, 252, 100, 236, 7, 172, 87, 170, 80, 207, 122, 181, 91, 120, 31, 198, 72, 62, 9, 54, 24, 114, 208, 200, 16, 126, 237, 6, 101, 43, 79, 108, 255, 88, 254, 188, 218, 124, 116, 214, 182, 25, 219, 28, 183, 227, 101, 197, 44, 71])
  ); // cxWg5RTK5AiSbBZh7NRg5btsbSrc8ETLXGf7tk3MUez

  const tx = new Transaction();
  tx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: mint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await provider.connection.getMinimumBalanceForRentExemption(
        SPLToken.MintLayout.span
      ),
      programId: TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      decimals ?? 6,
      authority,
      null// freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

  await provider.send(tx, [mint]);

  return mint.publicKey;
}

export const MINT_AUTHORITY = Keypair.fromSecretKey(
  Uint8Array.from([166, 35, 198, 106, 198, 244, 143, 224, 64, 125, 232, 144, 28, 45, 178, 146, 56, 92, 99, 244, 25, 75, 104, 247, 215, 33, 62, 30, 186, 249, 163, 48, 185, 210, 115, 123, 192, 235, 130, 28, 35, 27, 9, 65, 38, 210, 100, 190, 62, 225, 55, 90, 209, 0, 227, 160, 141, 54, 132, 242, 98, 240, 212, 95])
);

export const signer = Keypair.fromSecretKey(
  Uint8Array.from([97,46,44,175,15,110,7,237,243,15,55,50,158,227,91,232,109,165,63,244,59,126,23,13,93,71,241,70,180,56,221,33,142,67,104,248,208,129,43,80,134,141,191,238,249,147,90,77,210,45,251,174,145,27,89,173,190,201,123,173,222,199,92,207])
)

async function main() {

  const keypair = web3.Keypair.fromSeed(Uint8Array.from(keypairFile.slice(0, 32)))
  console.log('pubkey', keypair.publicKey.toString())
  const wallet = new anchor.Wallet(keypair)
  const connection = new web3.Connection('http://127.0.0.1:8899')
  const anchorProvider = new anchor.Provider(connection, wallet, {})
  anchor.setProvider(anchorProvider)

  console.log(MINT_AUTHORITY.publicKey.toString());

  const solanaProvider = SolanaProvider.init({
    connection,
    wallet,
    opts: {},
  })


  let cysMint = Keypair.fromSecretKey(
    Uint8Array.from([170, 204, 133, 206, 215, 135, 147, 69, 202, 136, 132, 212, 28, 149, 110, 252, 100, 236, 7, 172, 87, 170, 80, 207, 122, 181, 91, 120, 31, 198, 72, 62, 9, 54, 24, 114, 208, 200, 16, 126, 237, 6, 101, 43, 79, 108, 255, 88, 254, 188, 218, 124, 116, 214, 182, 25, 219, 28, 183, 227, 101, 197, 44, 71])
  ); // cxWg5RTK5AiSbBZh7NRg5btsbSrc8ETLXGf7tk3MUez
  const cysTx = new Transaction();  
  cysTx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: wallet.publicKey,
      newAccountPubkey: cysMint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await SPLToken.Token.getMinBalanceRentForExemptMint(connection),
      programId: SPLToken.TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      SPLToken.TOKEN_PROGRAM_ID, // program id, always token program id
      cysMint.publicKey, // mint account public key
      6, // decimals
      MINT_AUTHORITY.publicKey, // mint authority (an auth to mint token)
      null // freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );
  cysTx.feePayer = solanaProvider.wallet.publicKey;
  cysTx.recentBlockhash = (await solanaProvider.connection.getLatestBlockhash()).blockhash;
  const txhash = await anchorProvider.send(cysTx, [cysMint])
  console.log(`txhash: ${txhash}`);
  // solanaProvider.connection.getAccountInfo()
  const data = await solanaProvider.connection.getAccountInfo(cysMint.publicKey);

  const merkleSdk = MerkleDistributorSDK.load({ provider: solanaProvider });

  // console.log("SDK: ", merkleSdk.);

    const kpOne = web3.Keypair.generate();
    const kpTwo = web3.Keypair.generate();
    const kpThree = web3.Keypair.generate();
    const kpFour = web3.Keypair.generate();
    const kpFive = web3.Keypair.generate();
    const allKps = [kpOne, kpTwo, kpThree, kpFour, kpFive];
    await Promise.all(
      allKps.map(async (kp) => {
        await solanaProvider.connection.requestAirdrop(
          kp.publicKey,
          LAMPORTS_PER_SOL
        );
      })
    );
    console.log("Done here")

    const claimAmountOne = new u64(100);
    const claimAmountTwo = new u64(101);
    const claimAmountThree = new u64(102);
    const claimAmountFour = new u64(103);
    const claimAmountFive = new u64(104);
    const tree = new BalanceTree([
      { account: kpOne.publicKey, amount: claimAmountOne },
      { account: kpTwo.publicKey, amount: claimAmountTwo },
      { account: kpThree.publicKey, amount: claimAmountThree },
      { account: kpFour.publicKey, amount: claimAmountFour },
      { account: kpFive.publicKey, amount: claimAmountFive }
    ]);
    // console.log(tree)
    const root = tree.getRoot()

    // const { distributor } = await createAndSeedDistributor(
    //   merkleSdk,
    //   MAX_NUM_NODES,
    //   MAX_TOTAL_CLAIM,
    //   tree.getRoot()
    // )
    // console.log("Wokring till distrib")

    const newDistributor = await merkleSdk.createDistributor({
      root,
      maxNumNodes: new u64(100),
      maxTotalClaim: new u64(100000000),
      tokenMint: cysMint.publicKey
    })
    console.log("DISTIB: ", newDistributor.distributor.toString());
    console.log("ATA: ", newDistributor.distributorATA.toString());
    let txBuild = newDistributor.tx.build();
    txBuild.recentBlockhash = (await solanaProvider.connection.getLatestBlockhash()).blockhash;
    txBuild.feePayer = solanaProvider.wallet.publicKey;
    const str = txBuild.serializeMessage().toString('base64');
    console.log(`https://explorer.solana.com/tx/inspector?message=${encodeURIComponent(str)}&cluster=custom`) 
    let txSig = await anchorProvider.send(txBuild, newDistributor.tx.signers);
    console.log(`New Distributor::=>  ${txSig}`);
    console.log("Wokring right")
    
    // Seed merkle distributor with tokens
  const ix = SPLToken.Token.createMintToInstruction(
    TOKEN_PROGRAM_ID,
    cysMint.publicKey,
    newDistributor.distributorATA,
    MINT_AUTHORITY.publicKey,
    [],
    new u64(100000000)
  );
  const txBuild1 = new Transaction();
  txBuild1.recentBlockhash = (await solanaProvider.connection.getLatestBlockhash()).blockhash
  txBuild1.feePayer = solanaProvider.wallet.publicKey
  txBuild1.add(ix)
  const str1 = txBuild1.serializeMessage().toString('base64');
  console.log(`https://explorer.solana.com/tx/inspector?message=${encodeURIComponent(str1)}&cluster=custom`) 
  let txSig1 = await anchorProvider.send(txBuild1, [MINT_AUTHORITY])
  console.log(`New Distributor Seeded::=>  ${txSig1}`);

  const distributorW = await merkleSdk.loadDistributor(newDistributor.distributor);
  console.log(distributorW.key.toString())
  await Promise.all(
    allKps.map(async (kp, index) => {
      const amount = new u64(100 + index);
      const proof = tree.getProof(index, kp.publicKey, amount);

      const tx = await distributorW.claim({
        index: new u64(index),
        amount,
        proof,
        claimant: kp.publicKey,
      });
      let txBuild = tx.build();
      txBuild.recentBlockhash = (await solanaProvider.connection.getLatestBlockhash()).blockhash;
      txBuild.feePayer = solanaProvider.wallet.publicKey;
      const str = txBuild.serializeMessage().toString('base64');
      console.log(`https://explorer.solana.com/tx/inspector?message=${encodeURIComponent(str)}&cluster=custom`) 
      let txSig = await solanaProvider.send(txBuild, tx.signers)
      console.log(`Verified::=>  ${txSig}`);

      const tokenAccountInfo = await getTokenAccount(
        solanaProvider,
        await getATAAddress({
          mint: distributorW.data.mint,
          owner: kp.publicKey,
        })
      );
    })
  )
}

main().then(
  () => process.exit(),
  (err) => {
    console.error(err)
    process.exit(-1)
  }
)

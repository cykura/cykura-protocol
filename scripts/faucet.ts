import { Keypair, Connection, Transaction, SystemProgram } from "@solana/web3.js";

import * as SPLToken from "@solana/spl-token";

export const FEE_PAYER = Keypair.fromSecretKey(
  Uint8Array.from([166, 35, 198, 106, 198, 244, 143, 224, 64, 125, 232, 144, 28, 45, 178, 146, 56, 92, 99, 244, 25, 75, 104, 247, 215, 33, 62, 30, 186, 249, 163, 48, 185, 210, 115, 123, 192, 235, 130, 28, 35, 27, 9, 65, 38, 210, 100, 190, 62, 225, 55, 90, 209, 0, 227, 160, 141, 54, 132, 242, 98, 240, 212, 95])
);

export const CONNECTION = new Connection("http://localhost:8899");
// export const CONNECTION = new Connection("https://api.devnet.solana.com");

// Only One Time run

// Airdrop FIRST 
(async () => {
  // 1e9 lamports = 10^9 lamports = 1 SOL
  let txhash = await CONNECTION.requestAirdrop(FEE_PAYER.publicKey, 1e9);
  console.log(`txhash: ${txhash}`);
})();

// may fail because airdrop not completed
async function main() {

  let tx = new Transaction()

  // create a mint account
  let USDCmint = Keypair.fromSecretKey(
    Uint8Array.from([32, 171, 131, 168, 70, 59, 174, 186, 109, 21, 146, 106, 174, 39, 111, 122, 172, 195, 236, 162, 56, 12, 170, 173, 130, 146, 52, 31, 130, 238, 57, 203, 237, 74, 12, 237, 47, 252, 33, 48, 134, 162, 40, 246, 85, 115, 229, 218, 133, 17, 177, 158, 113, 216, 69, 157, 123, 177, 169, 46, 113, 4, 145, 52])
  );
  console.log(`USDCmint: ${USDCmint.publicKey.toString()}`);

  tx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: FEE_PAYER.publicKey,
      newAccountPubkey: USDCmint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await SPLToken.Token.getMinBalanceRentForExemptMint(CONNECTION),
      programId: SPLToken.TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      SPLToken.TOKEN_PROGRAM_ID, // program id, always token program id
      USDCmint.publicKey, // mint account public key
      6, // decimals
      FEE_PAYER.publicKey, // mint authority (an auth to mint token)
      null // freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );

  // ______________________________________________________
  let USDTmint = Keypair.fromSecretKey(
    Uint8Array.from([83, 68, 240, 117, 3, 161, 203, 18, 49, 31, 14, 135, 35, 13, 185, 79, 161, 190, 89, 119, 225, 79, 130, 251, 163, 211, 67, 245, 4, 147, 11, 71, 93, 124, 28, 237, 144, 117, 35, 92, 121, 21, 133, 203, 3, 117, 112, 81, 12, 127, 29, 104, 8, 138, 215, 207, 18, 92, 50, 227, 201, 220, 186, 255])
  );
  console.log(`USDTmint: ${USDTmint.publicKey.toString()}`);
  tx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: FEE_PAYER.publicKey,
      newAccountPubkey: USDTmint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await SPLToken.Token.getMinBalanceRentForExemptMint(CONNECTION),
      programId: SPLToken.TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      SPLToken.TOKEN_PROGRAM_ID, // program id, always token program id
      USDTmint.publicKey, // mint account public key
      6, // decimals
      FEE_PAYER.publicKey, // mint authority (an auth to mint token)
      null // freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );

  // _______________________________________________________________
  let SOLmint = Keypair.fromSecretKey(
    Uint8Array.from([185, 37, 210, 128, 228, 112, 57, 194, 25, 45, 254, 231, 202, 43, 240, 231, 235, 112, 90, 115, 140, 196, 119, 207, 200, 210, 5, 65, 179, 142, 36, 183, 195, 250, 100, 85, 223, 26, 50, 222, 66, 60, 147, 169, 91, 206, 29, 69, 125, 171, 222, 36, 249, 53, 6, 200, 211, 228, 96, 49, 135, 227, 98, 248])
  );
  console.log(`SOLmint: ${SOLmint.publicKey.toString()}`);
  tx.add(
    // create account
    SystemProgram.createAccount({
      fromPubkey: FEE_PAYER.publicKey,
      newAccountPubkey: SOLmint.publicKey,
      space: SPLToken.MintLayout.span,
      lamports: await SPLToken.Token.getMinBalanceRentForExemptMint(CONNECTION),
      programId: SPLToken.TOKEN_PROGRAM_ID,
    }),
    // init mint
    SPLToken.Token.createInitMintInstruction(
      SPLToken.TOKEN_PROGRAM_ID, // program id, always token program id
      SOLmint.publicKey, // mint account public key
      9, // decimals
      FEE_PAYER.publicKey, // mint authority (an auth to mint token)
      null // freeze authority (we use null first, the auth can let you freeze user's token account)
    )
  );
  tx.feePayer = FEE_PAYER.publicKey;

  let txhash = await CONNECTION.sendTransaction(tx, [USDCmint, USDTmint, SOLmint, FEE_PAYER]);
  console.log(`txhash: ${txhash}`);
}

main().then(
  () => process.exit(),
  (err) => {
    console.error(err);
    process.exit(-1);
  }
);
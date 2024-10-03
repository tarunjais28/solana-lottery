import {
  Connection,
  GetProgramAccountsConfig,
  Keypair,
  Signer,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";

import {
  AccountLayout,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  Token,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import fs from "fs";

const adminKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/admin.json")
        .toString()
    )
  )
);

async function main() {
  const rpc = "https://api.devnet.solana.com";
  const connection = new Connection(rpc);

  const args = process.argv.slice(2);
  const destPubkey = new PublicKey(args[0]);

  // const mint = await Token.createMint(
  //   connection,
  //   adminKp,
  //   adminKp.publicKey,
  //   null,
  //   6,
  //   TOKEN_PROGRAM_ID
  // );
  // console.log(mint.publicKey.toBase58());
  // 7tWUTDppUCLm482XrHqZK5mqChepjVdWw6xAkGXRBLeC

  const mint = new PublicKey("7tWUTDppUCLm482XrHqZK5mqChepjVdWw6xAkGXRBLeC");
  const token = new Token(connection, mint, TOKEN_PROGRAM_ID, adminKp);
  const tokenAI = await token.getOrCreateAssociatedAccountInfo(destPubkey);

  await token.mintTo(tokenAI.address, adminKp, [], 5 * Math.pow(10, 11));
}

main()
  .catch((err) => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());

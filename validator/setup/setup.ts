import {
  BpfLoader,
  BPF_LOADER_PROGRAM_ID,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import fs from "fs";
import {
  init,
} from "./staking/instructions"

import { sendAndConfirmTxWithSigners } from "./utils/utils";

const USDC_FACTOR = 1000_000;
const USDC_DECIMALS = 6;

const programKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/program.json")
        .toString()
    )
  )
);
const stakingProgramId = programKp.publicKey;

const nezhaVrfMockKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/nezha-vrf.json")
        .toString()
    )
  )
);
const vrfProgramId = nezhaVrfMockKp.publicKey;


const superAdminKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/superadmin.json")
        .toString()
    )
  )
);

const adminKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/admin.json")
        .toString()
    )
  )
);

const usdcMintKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/usdc.json")
        .toString()
    )
  )
);

const userKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/user.json")
        .toString()
    )
  )
);

const investorKp = Keypair.fromSecretKey(
  new Uint8Array(
    JSON.parse(
      fs
        .readFileSync("./keys/investor.json")
        .toString()
    )
  )
);

const SOL_AMOUNT = 1000;

async function main() {
  const rpc = process.env.SOLANA_RPC_URL as string;
  const connection = new Connection(rpc, 'confirmed');

  const superAdminPubkey = superAdminKp.publicKey;
  const superAdminAirdropTx = await connection.requestAirdrop(superAdminPubkey, SOL_AMOUNT * LAMPORTS_PER_SOL);
  console.log('Airdropping ' + SOL_AMOUNT + ' SOL to ' + superAdminPubkey);
  await connection.confirmTransaction(superAdminAirdropTx);

  const adminPubkey = adminKp.publicKey;
  const adminAirdropTx = await connection.requestAirdrop(adminPubkey, SOL_AMOUNT * LAMPORTS_PER_SOL);
  console.log('Airdropping ' + SOL_AMOUNT + ' SOL to ' + adminPubkey);
  await connection.confirmTransaction(adminAirdropTx);

  console.log('Creating mint ' + usdcMintKp.publicKey);
  const mint = await createMint(connection, adminKp, adminKp.publicKey, adminKp.publicKey, USDC_DECIMALS, usdcMintKp);

  console.log('Loading Nezha staking program');
  const programPath = process.argv[2] as string;
  const programData: Buffer = fs.readFileSync(programPath);
  await BpfLoader.load(connection, adminKp, programKp, programData, BPF_LOADER_PROGRAM_ID);
  const vrfProgramPath = process.argv[3] as string;
  const vrfProgramData: Buffer = fs.readFileSync(vrfProgramPath);
  await BpfLoader.load(connection, adminKp, nezhaVrfMockKp, vrfProgramData, BPF_LOADER_PROGRAM_ID);

  // init 
  console.log('Initializing contract');
  const insInitVault = await init(
    stakingProgramId,
    superAdminKp.publicKey,
    adminKp.publicKey,
    investorKp.publicKey,
    mint,
    vrfProgramId,
  )
  await sendAndConfirmTxWithSigners(connection, [insInitVault], [superAdminKp]);

  const userPubkey = userKp.publicKey;
  const userAirdropTx = await connection.requestAirdrop(userPubkey, SOL_AMOUNT * LAMPORTS_PER_SOL);
  console.log('Airdropping ' + SOL_AMOUNT + ' SOL to ' + userPubkey);
  await connection.confirmTransaction(userAirdropTx);

  const investorPubkey = investorKp.publicKey;
  const investorAirdropTx = await connection.requestAirdrop(investorPubkey, SOL_AMOUNT * LAMPORTS_PER_SOL);
  console.log('Airdropping ' + SOL_AMOUNT + ' SOL to ' + investorPubkey);
  await connection.confirmTransaction(investorAirdropTx);

  const adminTokenAccount = await getOrCreateAssociatedTokenAccount(connection, adminKp, mint, adminPubkey);
  const mintToAdminTx = await mintTo(connection, adminKp, mint, adminTokenAccount.address, adminKp, 1000000 * USDC_FACTOR);
  console.log('Minting fake USDC to ' + adminPubkey);
  await connection.confirmTransaction(mintToAdminTx);

  const userTokenAccount = await getOrCreateAssociatedTokenAccount(connection, adminKp, mint, userPubkey);
  const mintToUserTx = await mintTo(connection, adminKp, mint, userTokenAccount.address, adminKp, 1000000 * USDC_FACTOR);
  console.log('Minting fake USDC to ' + userPubkey);
  await connection.confirmTransaction(mintToUserTx);

  const investorTokenAccount = await getOrCreateAssociatedTokenAccount(connection, adminKp, mint, investorPubkey);
  const mintToInvestorTx = await mintTo(connection, adminKp, mint, investorTokenAccount.address, adminKp, 1000000 * USDC_FACTOR);
  console.log('Minting fake USDC to ' + investorPubkey);
  await connection.confirmTransaction(mintToInvestorTx);
}

main()
  .catch((err) => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());

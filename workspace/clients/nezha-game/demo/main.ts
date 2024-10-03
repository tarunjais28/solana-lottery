import * as fs from 'fs';
import * as path from 'path';
import * as url from 'url';

import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import { Connection, Keypair } from '@solana/web3.js';

import { USDC_MINT } from './constants.js';
import { STAKING_PROGRAM_ID } from './constants.js';
import {
  init,
  requestStakeUpdate,
  cancelStakeUpdate,
  claimWinning,
} from '../staking/instructions.js';
import { sendAndConfirmTxWithSigners } from '../utils/utils.js';
import { getStakeUpdateRequest } from '../staking/state_utils.js';

function loadKey(name: string): Keypair {
  let __filename = url.fileURLToPath(import.meta.url);
  let __dirname = path.dirname(__filename);
  let file = path.join(__dirname, 'keys', name + '.json');

  let s = fs.readFileSync(file).toString();
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(s)));
}

async function main() {
  const rpc = 'https://api.devnet.solana.com';
  const connection = new Connection(rpc);

  process.argv.shift();
  process.argv.shift();

  let cmd = process.argv.shift();
  if (cmd == 'init') {
    let superAdminKp = loadKey('super_admin');
    let adminKp = loadKey('admin');
    let investorKp = loadKey('investor');
    let ixn = await init(
      STAKING_PROGRAM_ID,
      superAdminKp.publicKey,
      adminKp.publicKey,
      investorKp.publicKey,
      USDC_MINT,
    );
    sendAndConfirmTxWithSigners(connection, [ixn], [superAdminKp]);
  } else if (cmd == 'deposit' || cmd == 'withdraw') {
    if (process.argv.length != 1) {
      console.log(`Syntax: ${cmd} <amount>`);
      return;
    }
    let amount = parseInt(process.argv.shift() as string);
    if (amount <= 0) {
      console.log('Error: amount must be greater than zero');
    }

    let userKp = loadKey('user');
    let userUSDC = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      USDC_MINT,
      userKp.publicKey,
    );

    if (cmd == 'withdraw') amount = amount * -1;

    let ixn = await requestStakeUpdate(STAKING_PROGRAM_ID, userKp.publicKey, userUSDC, amount);
    sendAndConfirmTxWithSigners(connection, [ixn], [userKp]);
  } else if (cmd == 'cancel_deposit' || cmd == 'cancel_withdraw') {

    let userKp = loadKey('user');
    let userUSDC = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      USDC_MINT,
      userKp.publicKey,
    );

    let stakeUpdateRequest = await getStakeUpdateRequest(connection, STAKING_PROGRAM_ID, userKp.publicKey);
    if (stakeUpdateRequest == null) {
      console.log("Active stake update request not found for the given user");
      return;
    }

    let amount = stakeUpdateRequest.amount;
    let ixn = await cancelStakeUpdate(STAKING_PROGRAM_ID, userKp.publicKey, userUSDC, amount);
    sendAndConfirmTxWithSigners(connection, [ixn], [userKp]);
  } else if (cmd == 'claim') {
    if (process.argv.length != 4) {
      console.log(`Syntax: ${cmd} <epoch-index> <page> <winner-index> <tier>`);
      return;
    }
    let [epochIndex, page, winnerIndex, tier] = [
      parseInt(process.argv[0] as string),
      parseInt(process.argv[1] as string),
      parseInt(process.argv[2] as string),
      parseInt(process.argv[3] as string),
    ];
    let userKp = loadKey('user');
    let ixn = await claimWinning(
      STAKING_PROGRAM_ID,
      userKp.publicKey,
      epochIndex,
      page,
      winnerIndex,
      tier,
    );
    sendAndConfirmTxWithSigners(connection, [ixn], [userKp]);
  } else {
    console.log(`Usage: main.ts <cmd> <args..>`);
    console.log();
    console.log('Commands:');
    console.log('  init');
    console.log('    Initialize the contract.');
    console.log('    Run by the super admin the first time the contract is deployed.');
    console.log('  deposit <amount>');
    console.log('     Stake USDC into the contract. `keys/user.json` will be used as the wallet.');
    console.log('  withdraw <amount>');
    console.log('     Withdraw USDC from the contract.');
    console.log('  cancel_deposit');
    console.log('     Cancel deposit request.');
    console.log('  cancel_withdraw');
    console.log('     Cancel withdraw request.');
    console.log('  claim <epoch-index> <page> <winner-index> <tier>');
    console.log('     Claim a prize for the user.');
    console.log('     The parameters required can be obtained from the API');
    console.log('     that returns the list of prizes to claim.');
  }
}

await main();

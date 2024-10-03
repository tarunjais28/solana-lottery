import {
  Connection,
  PublicKey,
  Signer,
  TransactionInstruction,
  Transaction,
} from "@solana/web3.js";
import BN from "bn.js";

// TODO option params
export async function sendAndConfirmTxWithSigners(
  connection: Connection,
  instructions: TransactionInstruction[],
  signers: Signer[]
) {
  let transaction = new Transaction();
  instructions.forEach((instruction) => transaction.add(instruction));
  transaction.recentBlockhash = (
    await connection.getLatestBlockhash()
  ).blockhash;
  const tx = await connection.sendTransaction(transaction, signers);
  console.log(`sending tx: ${tx}`);
  await connection.confirmTransaction(tx);
}

export function createInstruction(
  programId: PublicKey,
  keys: { pubkey: PublicKey; isSigner: boolean; isWritable: boolean }[],
  dataArray: number[]
) {
  return new TransactionInstruction({
    programId,
    keys,
    data: Buffer.from(new Uint8Array(dataArray)),
  });
}

export function signerKey(pubkey: PublicKey) {
  return {
    pubkey,
    isSigner: true,
    isWritable: true,
  };
}

export function writableKey(pubkey: PublicKey) {
  return {
    pubkey,
    isSigner: false,
    isWritable: true,
  };
}

export function readonlyKey(pubkey: PublicKey) {
  return {
    pubkey,
    isSigner: false,
    isWritable: false,
  };
}

export function numToU8Array(num: number) {
  return new BN(num).toArray("le", 8);
}

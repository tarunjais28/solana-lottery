import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, Transaction } from '@solana/web3.js';
import { PayerTransactionHandler } from '@metaplex-foundation/amman-client';

import debug from 'debug';
import test from 'tape';
import { clusterApiUrl } from '@solana/web3.js';
import { LOCALHOST } from '@metaplex-foundation/amman';
import { createTokenAccount } from './tokenAccount';
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore createMintToInstruction export actually exist but isn't setup correctly
import { createMintToInstruction } from '@solana/spl-token';
import { CreateMint } from './createMintAccount';

export { sleep } from './sleep';
export { createAndSignTransaction } from './createAndSignTransaction';

export const logDebug = debug('mpl:fp-test:debug');

export const DEVNET = clusterApiUrl('devnet');
export const connectionURL = process.env.USE_DEVNET != null ? DEVNET : LOCALHOST;

export function killStuckProcess() {
  // solana web socket keeps process alive for longer than necessary which we
  // "fix" here
  test.onFinish(() => process.exit(0));
}

export type TestContext = {
  payer: Keypair,
  connection: Connection,
  txHandler: PayerTransactionHandler,
}

export async function createPrerequisites(t: test.Test): Promise<TestContext> {
  const payer = Keypair.generate();

  const connection = new Connection(connectionURL, 'confirmed');
  const txHandler = new PayerTransactionHandler(connection, payer);

  const signature = await connection.requestAirdrop(payer.publicKey, 30 * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(signature);

  return { payer, connection, txHandler };
};

export async function airdrop(ctx: TestContext, acc: PublicKey, amount: number = 30) {

  const signature = await ctx.connection.requestAirdrop(acc, amount * LAMPORTS_PER_SOL);
  await ctx.connection.confirmTransaction(signature);
}

export async function prepareToken(t: test.Test, ctx: TestContext, owner: PublicKey, mint: Keypair, amount: number) {
  const mintAcc = await ctx.connection.getAccountInfo(mint.publicKey);
  if (mintAcc == null) {
    const { mint: _, createMintTx } = await CreateMint.createMintAccount(ctx, 6, mint);
    const tx = new Transaction();
    tx.feePayer = ctx.payer.publicKey;
    tx.add(createMintTx)
    await ctx.txHandler.sendAndConfirmTransaction(tx, [mint, ctx.payer], { skipPreflight: false }).assertSuccess(t);
  }

  const token =
    await createTokenAccount({ ctx, mint: mint.publicKey, owner });
  const mintToIx = createMintToInstruction(mint.publicKey, token.tokenAccount, ctx.payer.publicKey, amount);

  const tx = new Transaction();
  tx.add(
    ...token.createTokenTx.instructions,
    mintToIx,
  );

  await ctx.txHandler.sendAndConfirmTransaction(tx, [ctx.payer], { skipPreflight: false }).assertSuccess(t);
}

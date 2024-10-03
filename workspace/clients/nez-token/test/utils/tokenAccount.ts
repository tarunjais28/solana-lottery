import { PublicKey, Transaction } from '@solana/web3.js';
import { TestContext } from '.';
// @ts-ignore
import { createAssociatedTokenAccountInstruction } from '@solana/spl-token';
import { findAssociatedTokenAccount } from '../../src';

export const createTokenAccount = async ({
  ctx,
  mint,
  owner,
}: {
  ctx: TestContext;
  mint: PublicKey;
  owner: PublicKey;
}) => {
  const [tokenAccount] = await findAssociatedTokenAccount(owner!, mint);

  const createTokenTx = new Transaction();

  createTokenTx.add(
    createAssociatedTokenAccountInstruction(
      ctx.payer.publicKey, // payer
      tokenAccount,
      owner || ctx.payer.publicKey, // owner
      mint// mint
    )
  );

  createTokenTx.recentBlockhash = (await ctx.connection.getLatestBlockhash()).blockhash;
  createTokenTx.feePayer = ctx.payer.publicKey;

  return {
    tokenAccount,
    createTokenTx,
  };
};

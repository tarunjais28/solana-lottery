import { Keypair, Transaction } from '@solana/web3.js';
import test from 'tape';
import { airdrop, createPrerequisites, killStuckProcess, prepareToken, sleep } from './utils';
import { createDepositIx, createWithdrawIx, loadUserBalance } from '../src/types/stake';


killStuckProcess();

test('Stake', async function(t) {

  const ctx = await createPrerequisites(t);


  t.test('Deposit and Withdraw', async (t) => {
    const amount = 1000_000_000;
    const mint = Keypair.generate();
    const owner = Keypair.generate();
    airdrop(ctx, owner.publicKey);

    await prepareToken(t, ctx, owner.publicKey, mint, amount);

    const depositTx = new Transaction();
    depositTx.add(
      ...await createDepositIx(owner.publicKey, mint.publicKey, amount),
    );
    await ctx.txHandler.sendAndConfirmTransaction(depositTx, [owner], { skipPreflight: false }).assertSuccess(t);

    const userBalance = await loadUserBalance(ctx, owner.publicKey, mint.publicKey);
    t.equals(userBalance.amount.toString(), amount.toString());
    t.equals(userBalance.owner.toBase58(), owner.publicKey.toBase58());
    t.equals(userBalance.mint.toBase58(), mint.publicKey.toBase58());

    const withdrawTx = new Transaction();
    withdrawTx.add(
      ...await createWithdrawIx(owner.publicKey, mint.publicKey, amount),
    );
    await sleep(1000);
    await ctx.txHandler.sendAndConfirmTransaction(withdrawTx, [owner], { skipPreflight: false }).assertSuccess(t);

    const userBalanceAfter = await loadUserBalance(ctx, owner.publicKey, mint.publicKey);
    t.equals(userBalanceAfter.amount.toString(), "0");
    t.equals(userBalanceAfter.owner.toBase58(), owner.publicKey.toBase58());
    t.equals(userBalanceAfter.mint.toBase58(), mint.publicKey.toBase58());
  });


  t.test('Withdraw over balance', async (t) => {
    const amount = 1000_000_000;
    const mint = Keypair.generate();
    const owner = Keypair.generate();
    airdrop(ctx, owner.publicKey);

    await prepareToken(t, ctx, owner.publicKey, mint, amount);

    const depositTx = new Transaction();
    depositTx.add(
      ...await createDepositIx(owner.publicKey, mint.publicKey, amount),
    );
    await ctx.txHandler.sendAndConfirmTransaction(depositTx, [owner], { skipPreflight: false }).assertSuccess(t);

    const userBalance = await loadUserBalance(ctx, owner.publicKey, mint.publicKey);
    t.equals(userBalance.amount.toString(), amount.toString());
    t.equals(userBalance.owner.toBase58(), owner.publicKey.toBase58());
    t.equals(userBalance.mint.toBase58(), mint.publicKey.toBase58());

    const withdrawTx = new Transaction();
    withdrawTx.add(
      ...await createWithdrawIx(owner.publicKey, mint.publicKey, amount + 1),
    );
    await sleep(1000);
    await ctx.txHandler.sendAndConfirmTransaction(withdrawTx, [owner], { skipPreflight: true }).assertError(t);
  });

});




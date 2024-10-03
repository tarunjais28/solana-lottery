import { PublicKey, SYSVAR_CLOCK_PUBKEY, TransactionInstruction } from "@solana/web3.js";
import { createDepositInstruction as depositIx, createWithdrawInstruction as withdrawIx, UserBalance } from "../../src/generated";
import { findAssociatedTokenAccount, findUserBalanceAddress, findVaultAddress, findVaultTokenAddress } from "../../src/utils";
import { Context } from "../../src";


/**
 * Returns instructions for making a deposit into the user stake.  
 *
 * @signers owner
 */
export async function createDepositIx(owner: PublicKey, mint: PublicKey, amount: number): Promise<[TransactionInstruction]> {
  const [userBalance] = await findUserBalanceAddress(owner, mint);
  const [token] = await findAssociatedTokenAccount(owner, mint);
  const [vaultToken] = await findVaultTokenAddress(mint);
  const [vault] = await findVaultAddress();


  return [
    depositIx({
      owner,
      userBalance,
      token,
      mint,
      vaultToken,
      vault,
      clock: SYSVAR_CLOCK_PUBKEY
    }, {
      amount
    })
  ]
}

export async function loadUserBalance(ctx: Context, owner: PublicKey, mint: PublicKey): Promise<UserBalance> {

  const [userBalance] = await findUserBalanceAddress(owner, mint);
  const userBalanceData = await ctx.connection.getAccountInfo(userBalance);

  return UserBalance.deserialize(userBalanceData!.data)[0];
}


/**
 * Returns instructions for withdrawing amount from the user stake.  
 *
 * @signers owner
 */
export async function createWithdrawIx(owner: PublicKey, mint: PublicKey, amount: number): Promise<[TransactionInstruction]> {
  const [userBalance] = await findUserBalanceAddress(owner, mint);
  const [token] = await findAssociatedTokenAccount(owner, mint);
  const [vaultToken] = await findVaultTokenAddress(mint);
  const [vault] = await findVaultAddress();


  return [
    withdrawIx({
      owner,
      userBalance,
      token,
      vaultToken,
      vault,
      clock: SYSVAR_CLOCK_PUBKEY
    }, {
      amount
    })
  ]
}

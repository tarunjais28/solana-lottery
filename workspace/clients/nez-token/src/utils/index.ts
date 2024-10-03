import { PublicKey } from '@solana/web3.js';
import { PROGRAM_ID } from '../generated';
import { BALANCE_SEED, SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID, VAULT_SEED } from '../consts';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';


export async function findVaultAddress(
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [Buffer.from(VAULT_SEED)],
    PROGRAM_ID,
  );
}


export async function findVaultTokenAddress(
  mint: PublicKey,
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [Buffer.from(VAULT_SEED), mint.toBuffer()],
    PROGRAM_ID,
  );
}

export async function findUserBalanceAddress(
  owner: PublicKey,
  mint: PublicKey,
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddress(
    [Buffer.from(BALANCE_SEED), owner.toBuffer(), mint.toBuffer()],
    PROGRAM_ID,
  );
}

export async function findAssociatedTokenAccount(
  owner: PublicKey,
  mint: PublicKey
): Promise<[PublicKey, number]> {
  return (await PublicKey.findProgramAddress(
    [
      owner.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
  ));
}

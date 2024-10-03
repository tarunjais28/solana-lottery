import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
  AccountMeta,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from '@solana/web3.js';
import BN from 'bn.js';
import * as ac from './accounts.js';
import { concatBufs, u64, u32, u8, i64, i64BN } from './buffer_utils.js';

function instruction(
  program_id: PublicKey,
  keys: AccountMeta[],
  data: Buffer[],
): TransactionInstruction {
  return new TransactionInstruction({ programId: program_id, keys, data: concatBufs(data) });
}

enum Instruction {
  Init = 0,
  RequestStakeUpdate,
  ApproveStakeUpdate,
  CancelStakeUpdate,
  Removed3,
  Removed4,
  CreateEpoch,
  Removed1,
  CreateStakingTicket,
  DeclareEpochWinningCombination,
  ClaimWinning,
  YieldWithdrawByInvestor,
  YieldDepositByInvestor,
  FundJackpot,
  Removed2,
  FranciumInit,
  FranciumInvest,
  FranciumWithdraw,
  WithdrawVault,
  CompleteStakeUpdate,
  CreateEpochWinnersMeta,
  PublishWinners,
  RotateKey,
}

type KeyModifier = 'signer' | 'writable';

function key(mods: KeyModifier[], key: PublicKey): AccountMeta {
  let isSigner = false;
  let isWritable = false;

  for (let m of mods) {
    if (m == 'signer') isSigner = true;
    if (m == 'writable') isWritable = true;
  }

  return {
    pubkey: key,
    isSigner,
    isWritable,
  };
}

async function init(
  program_id: PublicKey,
  super_admin: PublicKey,
  admin: PublicKey,
  investor: PublicKey,
  usdc_mint: PublicKey,
): Promise<TransactionInstruction> {
  return instruction(
    program_id,
    [
      key(['signer', 'writable'], super_admin),
      key([], admin),
      key([], investor),
      key([], usdc_mint),
      key([], await ac.vault_authority(program_id)),
      key(['writable'], await ac.deposit_vault(program_id)),
      key(['writable'], await ac.treasury_vault(program_id)),
      key(['writable'], await ac.insurance_vault(program_id)),
      key(['writable'], await ac.prize_vault(program_id, 1)),
      key(['writable'], await ac.prize_vault(program_id, 2)),
      key(['writable'], await ac.prize_vault(program_id, 3)),
      key(['writable'], await ac.pending_deposit_vault(program_id)),
      key(['writable'], await ac.latest_epoch(program_id)),
      key([], SystemProgram.programId),
      key([], TOKEN_PROGRAM_ID),
      key([], SYSVAR_RENT_PUBKEY),
    ],
    [u8(Instruction.Init)],
  );
}

//

async function requestStakeUpdate(
  program_id: PublicKey,
  owner: PublicKey,
  owner_usdc_token: PublicKey,
  amount: number,
): Promise<TransactionInstruction> {
  return instruction(
    program_id,
    [
      key(['signer', 'writable'], owner),
      key(['writable'], owner_usdc_token),
      key(['writable'], await ac.stake_update_request(program_id, owner)),
      key(['writable'], await ac.pending_deposit_vault(program_id)),
      key([], SystemProgram.programId),
      key([], TOKEN_PROGRAM_ID),
      key([], SYSVAR_RENT_PUBKEY),
    ],
    [u8(Instruction.RequestStakeUpdate), i64(amount)],
  );
}

async function cancelStakeUpdate(
  program_id: PublicKey,
  owner: PublicKey,
  owner_usdc_token: PublicKey,
  amount: BN
): Promise<TransactionInstruction> {
  return instruction(
    program_id,
    [
      key(['signer', 'writable'], owner),
      key([], owner),
      key(['writable'], owner_usdc_token),
      key(['writable'], await ac.stake_update_request(program_id, owner)),
      key(['writable'], await ac.pending_deposit_vault(program_id)),
      key([], await ac.vault_authority(program_id)),
      key([], await ac.latest_epoch(program_id)),
      key([], TOKEN_PROGRAM_ID),
    ],
    [u8(Instruction.CancelStakeUpdate), i64BN(amount)],
  );
}

async function claimWinning(
  program_id: PublicKey,
  owner: PublicKey,
  epoch_index: number,
  page: number,
  winner_index: number,
  tier: number,
): Promise<TransactionInstruction> {
  return instruction(
    program_id,
    [
      key([], owner),
      key(['writable'], await ac.epoch_winners_meta(program_id, epoch_index)),
      key(['writable'], await ac.epoch_winners_page(program_id, epoch_index, page)),
      key(['writable'], await ac.stake(program_id, owner)),
      key([], await ac.epoch(program_id, epoch_index)),
      key(['writable'], await ac.latest_epoch(program_id)),
      key([], await ac.vault_authority(program_id)),
      key(['writable'], await ac.prize_vault(program_id, tier)),
      key(['writable'], await ac.deposit_vault(program_id)),
      key([], TOKEN_PROGRAM_ID),
    ],
    [u8(Instruction.ClaimWinning), u64(epoch_index), u32(page), u32(winner_index), u8(tier)],
  );
}

export { init, requestStakeUpdate, cancelStakeUpdate, claimWinning };

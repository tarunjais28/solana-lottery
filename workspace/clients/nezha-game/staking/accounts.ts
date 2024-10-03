import { PublicKey } from '@solana/web3.js';
import { u8, u32, u64 } from './buffer_utils.js';

const PREFIX = 'staking';

async function pda(
  program_id: PublicKey,
  seeds: (string | Buffer | PublicKey)[],
): Promise<PublicKey> {
  let seeds_bytes: Buffer[] = [];
  for (let s of seeds) {
    if (typeof s == 'string') {
      seeds_bytes.push(Buffer.from(s));
    } else if (s instanceof Buffer) {
      seeds_bytes.push(s);
    } else {
      seeds_bytes.push(s.toBuffer());
    }
  }
  let [key, _bump] = await PublicKey.findProgramAddress(seeds_bytes, program_id);
  return key;
}

export async function latest_epoch(program_id: PublicKey) {
  return await pda(program_id, [PREFIX, 'LATEST_EPOCH']);
}

export async function epoch(program_id: PublicKey, index: number): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'EPOCH', u64(index)]);
}

export async function epoch_tier_winner(
  program_id: PublicKey,
  index: number,
  tier: number,
): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'EPOCH', u64(index), 'TIER_WINNERS', u8(tier)]);
}

export async function stake(program_id: PublicKey, owner: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'STAKE', owner]);
}

export async function stake_update_request(
  program_id: PublicKey,
  owner: PublicKey,
): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'STAKE_UPDATE_REQUEST', owner]);
}

export async function staking_ticket(
  program_id: PublicKey,
  epoch_index: number,
  owner: PublicKey,
): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'STAKING_TICKET', u64(epoch_index), owner]);
}

export async function vault_authority(program_id: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT_AUTHORITY']);
}

export async function deposit_vault(program_id: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT', 'DEPOSIT']);
}

export async function treasury_vault(program_id: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT', 'TREASURY']);
}

export async function insurance_vault(program_id: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT', 'INSURANCE']);
}

export async function prize_vault(program_id: PublicKey, tier: number): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT', 'PRIZE', u8(tier)]);
}

export async function pending_deposit_vault(program_id: PublicKey): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'VAULT', 'PENDING_DEPOSIT']);
}

export async function epoch_winners_meta(
  program_id: PublicKey,
  epoch_index: number,
): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'EPOCH_WINNERS_META', u64(epoch_index)]);
}

export async function epoch_winners_page(
  program_id: PublicKey,
  epoch_index: number,
  page: number,
): Promise<PublicKey> {
  return await pda(program_id, [PREFIX, 'EPOCH_WINNERS_PAGE', u64(epoch_index), u32(page)]);
}

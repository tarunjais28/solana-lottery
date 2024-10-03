import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { AccountMeta, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, TransactionInstruction } from "@solana/web3.js"
import * as ac from "./accounts";
import { concatBufs, u64, u8 } from "./buffer_utils";

function instruction(program_id: PublicKey, keys: AccountMeta[], data: Buffer[]): TransactionInstruction {
	return new TransactionInstruction({ programId: program_id, keys, data: concatBufs(data) });
}

enum Instruction {
	Init = 0,
	AttemptDeposit,
	ApproveDeposit,
	CancelDepositAttempt,
	CloseDepositAttemptByAdmin,
	WithdrawStake,
	CreateEpoch,
	Removed1,
	CreateStakingTicket,
	DeclareEpochWinningCombination,
	ClaimWinning,
	YieldWithdrawByInvestor,
	YieldDepositByInvestor,
	FundJackpot,
}

const key = {
	signerWritable: (key: PublicKey) => ({
		pubkey: key,
		isSigner: true,
		isWritable: true,
	}),
	signer: (key: PublicKey) => ({
		pubkey: key,
		isSigner: true,
		isWritable: false,
	}),
	writable: (key: PublicKey) => ({
		pubkey: key,
		isSigner: false,
		isWritable: true,
	}),
	readOnly: (key: PublicKey) => ({
		pubkey: key,
		isSigner: false,
		isWritable: false,
	}),
}

export async function init(program_id: PublicKey, superadmin: PublicKey, admin: PublicKey, investor: PublicKey, usdc_mint: PublicKey): Promise<TransactionInstruction> {
	return instruction(
		program_id,
		[
			key.signerWritable(superadmin),
			key.readOnly(admin),
			key.readOnly(investor),
			key.readOnly(usdc_mint),
			key.readOnly(await ac.vault_authority(program_id)),
			key.writable(await ac.deposit_vault(program_id)),
			key.writable(await ac.treasury_vault(program_id)),
			key.writable(await ac.insurance_vault(program_id)),
			key.writable(await ac.prize_vault(program_id, 1)),
			key.writable(await ac.prize_vault(program_id, 2)),
			key.writable(await ac.prize_vault(program_id, 3)),
			key.writable(await ac.pending_deposit_vault(program_id)),
			key.writable(await ac.latest_epoch(program_id)),
			key.readOnly(SystemProgram.programId),
			key.readOnly(TOKEN_PROGRAM_ID),
			key.readOnly(SYSVAR_RENT_PUBKEY),
		],
		[
			u8(Instruction.Init),
		]
	)
}


// 

export async function attemptDeposit(program_id: PublicKey, owner: PublicKey, owner_usdc_token: PublicKey, amount: number): Promise<TransactionInstruction> {
	return instruction(
		program_id,
		[
			key.signerWritable(owner),
			key.writable(owner_usdc_token),
			key.writable(await ac.deposit_attempt(program_id, owner)),
			key.readOnly(SystemProgram.programId),
			key.readOnly(TOKEN_PROGRAM_ID),
			key.readOnly(SYSVAR_RENT_PUBKEY),
		],
		[u8(Instruction.AttemptDeposit), u64(amount)]
	)
}

export async function cancelDepositAttempt(program_id: PublicKey, owner: PublicKey, owner_usdc_token: PublicKey): Promise<TransactionInstruction> {
	return instruction(
		program_id,
		[
			key.signerWritable(owner),
			key.writable(owner_usdc_token),
			key.writable(await ac.deposit_attempt(program_id, owner)),
			key.readOnly(TOKEN_PROGRAM_ID),
		],
		[u8(Instruction.CancelDepositAttempt)],
	)
}

export async function withdrawStake(
	program_id: PublicKey,
	owner: PublicKey,
	owner_usdc_token: PublicKey,
	amount: number,
): Promise<TransactionInstruction> {
	return instruction(
		program_id,
		[
			key.signer(owner),
			key.writable(owner_usdc_token),
			key.writable(await ac.stake(program_id, owner)),
			key.readOnly(await ac.latest_epoch(program_id)),
			key.readOnly(await ac.vault_authority(program_id)),
			key.writable(await ac.deposit_vault(program_id)),
			key.readOnly(TOKEN_PROGRAM_ID),
		],
		[u8(Instruction.WithdrawStake), u64(amount)]
	)
}

export async function claimWinning(
	program_id: PublicKey,
	owner: PublicKey,
	epoch_index: number,
	tier: number,
): Promise<TransactionInstruction> {
	return instruction(
		program_id,
		[
			key.readOnly(owner),
			key.writable(await ac.epoch_tier_winner(program_id, epoch_index, tier)),
			key.writable(await ac.stake(program_id, owner)),
			key.writable(await ac.latest_epoch(program_id)),
			key.readOnly(await ac.vault_authority(program_id)),
			key.writable(await ac.prize_vault(program_id, tier)),
			key.writable(await ac.deposit_attempt(program_id, owner)),
			key.readOnly(TOKEN_PROGRAM_ID),
		],
		[u8(Instruction.ClaimWinning), u8(tier)]
	)
}

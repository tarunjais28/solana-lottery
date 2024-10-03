import { Connection, PublicKey } from '@solana/web3.js';
import * as ac from './accounts.js';
import { StakeUpdateRequest } from './state.js';

async function getStakeUpdateRequest(connection: Connection, programId: PublicKey, userPubkey: PublicKey) {
	let accountPubkey = await ac.stake_update_request(programId, userPubkey);
	let accountInfo = await connection.getAccountInfo(accountPubkey);
	if (accountInfo == null) {
		return null;
	}

	let parsed = StakeUpdateRequest.deserialize(accountInfo.data);
	return parsed;
}

export { getStakeUpdateRequest };

import BN from "bn.js";
import borsh from 'borsh';

class StakeUpdateRequest {
	public accountType;
	public contractVersion;
	public isInitialized;
	public amount;
	public state;

	constructor(props: any) {
		this.accountType = props.accountType;
		this.contractVersion = props.contractVersion;
		this.isInitialized = props.isInitialized;
		this.amount = new BN(props.amount, 'le').fromTwos(64); // borsh encoding of integers is always little-endian
		this.state = props.state;
	}

	static deserialize(data: Buffer): StakeUpdateRequest {
		return borsh.deserialize(SCHEMA, StakeUpdateRequest, data);
	}
}

const SCHEMA = new Map([
	[StakeUpdateRequest,
		{
			kind: 'struct',
			fields: [
				['accountType', 'u8'],
				['contractVersion', 'u8'],
				['isInitialized', 'u8'], // Borsh in JS doesn't support bool as of Jul 2023.
				['owner', [32]],
				['amount', [8]],  // Borsh in JS doesn't support i64 as of Jul 2023.
				['state', 'u8'],
			]
		}
	]
]);

export { StakeUpdateRequest, SCHEMA }

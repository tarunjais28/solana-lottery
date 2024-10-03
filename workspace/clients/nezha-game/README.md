# Staking contract typescript client
## Usage
- First, create the contract instruction with create instruction helpers from `./staking/instructions.ts`
- Then, send the instruction with a transaction using the helper function `sendAndConfirmTxWithSigners` in `./utils/utils`

Example:
```
import { createAttemptDepositInstruction } from "./staking/instructions";
import { sendAndConfirmTxWithSigners } from "./utils/utils";

.
.
.

const ix = await createAttemptDepositInstruction(
  STAKING_PROGRAM_ID,
  user,
  userUsdcToken,
  amount
);
await sendAndConfirmTxWithSigners(connection, [ix], [userKp]);

```

## Instructions
### `createInitStakeInstruction`
Instruction for creating Stake account for "first timer" wallet
### `createAttemptDepositInstruction` 
Instruction for attempting to make deposit from the user
### `createApproveDepositInstruction`
Instruction for approving deposit after AML check passes
### `createWithdrawStakeInstruction`
Instruction for withdrawing from the staked deposit for user
### `createClaimWinningInstruction`
Instruction for user to claim winning for a certain tier of a certain epoch

use nezha_staking::error::StakingError;
use solana_client::client_error::ClientError;
use solana_program::instruction::InstructionError;
use solana_sdk::transaction::TransactionError;

pub fn decode_staking_error(err: &ClientError) -> Option<StakingError> {
    let txn_error = err.get_transaction_error();
    if let Some(error) = txn_error {
        if let TransactionError::InstructionError(_, error) = error {
            if let InstructionError::Custom(number) = error {
                return nezha_staking::error::StakingError::try_from(number).ok();
            }
        }
    }
    None
}

use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::impl_borsh_length;
use num_derive::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, FromPrimitive)]
pub enum AccountType {
    NotInitialized,
    NezhaVrfProgramState,
    NezhaVrfRequest,
    //
    SwitchboardVrfLite,
    SwitchboardAuthority,
    //
    NezhaStakingLatestEpoch = 100,
}

impl_borsh_length!(AccountType, 1);

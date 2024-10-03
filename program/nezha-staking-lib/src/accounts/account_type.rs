use borsh::{BorshDeserialize, BorshSerialize};
use num_derive::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, FromPrimitive)]
pub enum AccountType {
    // 0
    LatestEpoch,
    Epoch,
    Stake,
    StakeUpdateRequest,
    EpochWinnersMeta,
    EpochWinnersPage,
    //
    VaultAuthority,
    FranciumAuthority,
    //
    DepositVault,
    TreasuryVault,
    InsuranceVault,
    PendingDepositVault,
    Tier1PrizeVault,
    Tier2PrizeVault,
    Tier3PrizeVault,
    //
    NezhaVrfRequest = 100,
}

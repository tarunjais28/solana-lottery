use borsh::{BorshDeserialize, BorshSerialize};
use num_derive::FromPrimitive;

/// Epoch status.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Copy, FromPrimitive, Eq, PartialOrd, Ord)]
pub enum EpochStatus {
    /// Deposits/Withdraws are allowed
    Running = 0,
    /// Funds are moved into the investment platform. Any further deposits/withdraws will be queued
    /// till next epoch.
    Yielding,
    /// Funds are returned from the investment platform.
    /// Winning combination and winners list is being uploaded.
    Finalising,
    /// Winners are declared.
    Ended,
}

impl EpochStatus {
    pub fn as_display(&self) -> &'static str {
        match self {
            EpochStatus::Running => "Running",
            EpochStatus::Yielding => "Yielding",
            EpochStatus::Finalising => "Finalising",
            EpochStatus::Ended => "Ended",
        }
    }
}

impl Default for EpochStatus {
    fn default() -> Self {
        EpochStatus::Running
    }
}

impl EpochStatus {
    pub const fn max_len() -> usize {
        1
    }
}

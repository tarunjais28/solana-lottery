//! Definitions of all PDAs used by the contract.

use solana_program::pubkey::Pubkey;

#[cfg(test)]
mod tests;

mod account_type;
pub use account_type::AccountType;

mod verify;
pub use verify::VerifyPDA;

use nezha_utils::seeds;

pub type PDA = nezha_utils::pda::PDA<AccountType>;

pub const PREFIX: &str = "NEZHA_VRF";

/// Authority for interacting with switchboard
pub fn nezha_vrf_program_state(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "PROGRAM_STATE"),
        AccountType::NezhaVrfProgramState,
    )
}

/// VRF Request for an epoch
pub fn nezha_vrf_request(program_id: &Pubkey, epoch_index: u64) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VRF_REQUEST", epoch_index),
        AccountType::NezhaVrfRequest,
    )
}

/// Authority for interacting with switchboard
pub fn switchboard_authority(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "SWITCHBOARD", "AUTHORITY"),
        AccountType::SwitchboardAuthority,
    )
}

/// VRF Lite account for Switchboard
pub fn switchboard_vrf_lite(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "SWITCHBOARD", "VRF_LITE"),
        AccountType::SwitchboardVrfLite,
    )
}

use solana_program::pubkey::Pubkey;
use static_pubkey::static_pubkey;

pub use switchboard_v2::ID as SWITCHBOARD_PROGRAM_ID;

pub const SWITCHBOARD_LABS_DEVNET_PERMISSIONLESS_QUEUE: Pubkey =
    static_pubkey!("uPeRMdfPmrPqgRWSrjAnAkH78RqAhe5kXoW6vBYRqFX");

pub const SWITCHBOARD_LABS_MAINNET_PERMISSIONLESS_QUEUE: Pubkey =
    static_pubkey!("5JYwqvKkqp35w8Nq3ba4z1WYUeJQ1rB36V8XvaGp6zn1");

pub use anchor_lang::prelude::Error as AnchorError;
pub use anchor_lang::AccountDeserialize;
pub use switchboard_v2::OracleQueueAccountData;
pub use switchboard_v2::SbState;

pub fn get_permission_pda(
    switchboard_program_id: &Pubkey,
    queue_authority: &Pubkey,
    queue: &Pubkey,
    vrf_lite: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            switchboard_v2::PERMISSION_SEED,
            queue_authority.as_ref(),
            queue.as_ref(),
            vrf_lite.as_ref(),
        ],
        switchboard_program_id,
    )
}

pub fn get_program_state_pda(switchboard_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[switchboard_v2::STATE_SEED], switchboard_program_id)
}

pub fn deserialize_oracle_queue_account_data(mut data: &[u8]) -> Result<OracleQueueAccountData, AnchorError> {
    AccountDeserialize::try_deserialize(&mut data)
}

pub fn deserialize_sb_state(mut data: &[u8]) -> Result<SbState, AnchorError> {
    AccountDeserialize::try_deserialize(&mut data)
}

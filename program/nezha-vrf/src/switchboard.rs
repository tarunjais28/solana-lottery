use borsh::BorshSerialize;
use nezha_utils::account_meta;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
};
use switchboard_v2::Callback;

use crate::utils::{check_ata_program, check_rent_sysvar, check_system_program, check_token_program};
use nezha_utils::checks::{check_is_signer_writable, SignerWritable};

pub const VRF_LITE_INIT_DISCRIMINATOR: [u8; 8] = [144, 40, 147, 33, 238, 92, 88, 46];

pub struct VrfLiteInitAccounts<'a, 'b> {
    pub switchboard_program: AccountInfo<'a>,
    pub authority: AccountInfo<'a>,
    pub vrf_lite: AccountInfo<'a>,
    pub vrf_lite_seeds: Option<&'b [&'b [u8]]>,
    pub mint: AccountInfo<'a>,
    pub escrow: AccountInfo<'a>,
    pub queue_authority: AccountInfo<'a>,
    pub queue: AccountInfo<'a>,
    pub permission: AccountInfo<'a>,
    pub program_state: AccountInfo<'a>,
    pub payer: AccountInfo<'a>,
    //
    pub token_program: AccountInfo<'a>,
    pub ata_program: AccountInfo<'a>,
    pub system_program: AccountInfo<'a>,
    pub rent_sysvar: AccountInfo<'a>,
}

impl VrfLiteInitAccounts<'_, '_> {
    fn verify(&self) -> ProgramResult {
        check_is_signer_writable(
            "authority",
            &self.authority,
            &SignerWritable {
                is_signer: false,
                is_writable: false,
            },
        )?;
        check_is_signer_writable(
            "vrf_lite",
            &self.vrf_lite,
            &SignerWritable {
                is_signer: self.vrf_lite_seeds.is_none(),
                is_writable: true,
            },
        )?;
        check_is_signer_writable(
            "mint",
            &self.queue,
            &SignerWritable {
                is_signer: false,
                is_writable: false,
            },
        )?;
        check_is_signer_writable(
            "escrow",
            &self.escrow,
            &SignerWritable {
                is_signer: false,
                is_writable: true,
            },
        )?;
        check_is_signer_writable(
            "queue_authority",
            &self.queue_authority,
            &SignerWritable {
                is_signer: false,
                is_writable: false,
            },
        )?;
        check_is_signer_writable(
            "queue",
            &self.queue,
            &SignerWritable {
                is_signer: false,
                is_writable: false,
            },
        )?;
        check_is_signer_writable(
            "permission",
            &self.permission,
            &SignerWritable {
                is_signer: false,
                is_writable: true,
            },
        )?;
        check_is_signer_writable(
            "program_state",
            &self.program_state,
            &SignerWritable {
                is_signer: false,
                is_writable: false,
            },
        )?;
        check_is_signer_writable(
            "payer",
            &self.payer,
            &SignerWritable {
                is_signer: true,
                is_writable: true,
            },
        )?;

        check_token_program(&self.token_program)?;
        check_ata_program(&self.ata_program)?;
        check_system_program(&self.system_program)?;
        check_rent_sysvar(&self.rent_sysvar)?;

        Ok(())
    }
}

#[derive(BorshSerialize)]
pub struct VrfLiteInitParams {
    pub callback: Option<Callback>,
    pub state_bump: u8,
    pub expiration: Option<i64>,
}

impl VrfLiteInitParams {
    pub fn serialize(&self) -> Result<Vec<u8>, ProgramError> {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&VRF_LITE_INIT_DISCRIMINATOR);

        BorshSerialize::serialize(&self, &mut data)?;
        Ok(data)
    }
}

pub fn vrf_lite_init(params: &VrfLiteInitParams, accounts: &VrfLiteInitAccounts) -> ProgramResult {
    accounts.verify()?;
    let ixn_data = params.serialize()?;
    invoke_signed(
        &Instruction::new_with_bytes(
            *accounts.switchboard_program.key,
            &ixn_data,
            account_meta![
                [] *accounts.authority.key,
                [signer writable] *accounts.vrf_lite.key,
                [] *accounts.mint.key,
                [writable] *accounts.escrow.key,
                [] *accounts.queue_authority.key,
                [] *accounts.queue.key,
                [writable] *accounts.permission.key,
                [] *accounts.program_state.key,
                [signer writable] *accounts.payer.key,
                //
                [] *accounts.token_program.key,
                [] *accounts.ata_program.key,
                [] *accounts.system_program.key,
                [] *accounts.rent_sysvar.key,
            ],
        ),
        &[
            accounts.authority.clone(),
            accounts.vrf_lite.clone(),
            accounts.mint.clone(),
            accounts.escrow.clone(),
            accounts.queue_authority.clone(),
            accounts.queue.clone(),
            accounts.permission.clone(),
            accounts.program_state.clone(),
            accounts.payer.clone(),
            //
            accounts.token_program.clone(),
            accounts.ata_program.clone(),
            accounts.system_program.clone(),
            accounts.rent_sysvar.clone(),
        ],
        accounts
            .vrf_lite_seeds
            .as_ref()
            .map(core::slice::from_ref)
            .unwrap_or_default(),
    )?;
    Ok(())
}

#[test]
fn test_vrf_lite_init_params_serialize() {
    let params = VrfLiteInitParams {
        callback: None,
        state_bump: 10,
        expiration: None,
    };
    let data = params.serialize().unwrap();
    let mut data_expected = Vec::new();
    data_expected.extend_from_slice(&VRF_LITE_INIT_DISCRIMINATOR);
    data_expected.push(0); // callback: None
    data_expected.push(10); // state_bump: 10
    data_expected.push(0); // expiration: None
    assert_eq!(data, data_expected);
}

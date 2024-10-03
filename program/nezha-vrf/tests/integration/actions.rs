#![allow(dead_code)]

use borsh::BorshSerialize;
use nezha_staking_lib::accounts as staking_ac;
use nezha_staking_lib::state::CumulativeReturnRate;
use nezha_staking_lib::state::EpochStatus;
use nezha_staking_lib::state::LatestEpoch;
use nezha_vrf_lib::accounts as ac;
use nezha_vrf_lib::instruction;
use nezha_vrf_lib::state::HasAccountType;
use nezha_vrf_lib::state::NezhaVrfRequest;

use crate::accounts::Accounts;
use anyhow::Context;
use anyhow::Result;
use borsh::BorshDeserialize;
use solana_program::borsh0_10::try_from_slice_unchecked;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

use nezha_testing::solana_test_runtime::{Account, SolanaTestRuntime};

pub async fn create_mint(
    mint: &Keypair,
    manager: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let rent = processor.get_rent().await.unwrap();
    let payer = &processor.get_payer().pubkey();

    processor
        .send_ixns(
            &[
                system_instruction::create_account(
                    payer,
                    &mint.pubkey(),
                    rent.minimum_balance(spl_token::state::Mint::LEN),
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint.pubkey(),
                    &manager,
                    freeze_authority,
                    6,
                )?,
            ],
            &[&mint],
        )
        .await
}

pub async fn init(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::init(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                &accounts.admin.pubkey(),
                &switchboard_v2::ID,
                &accounts.switchboard_queue,
                &accounts.switchboard_queue_authority,
                &accounts.switchboard_queue_mint,
                &accounts.nezha_staking_program_id,
            )],
            &[&accounts.super_admin, &accounts.admin],
        )
        .await
}

pub async fn request_vrf(epoch_index: u64, accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::request_vrf(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &switchboard_v2::ID,
                &accounts.switchboard_queue,
                &accounts.switchboard_queue_authority,
                &accounts.switchboard_queue_mint,
                &accounts.switchboard_queue_data_buffer,
                &staking_ac::latest_epoch(&accounts.nezha_staking_program_id),
                epoch_index,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn consume_vrf(epoch_index: u64, accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::consume_vrf(&accounts.program_id, epoch_index)],
            &[&accounts.admin],
        )
        .await
}

pub async fn set_epoch_index_and_status(
    epoch_index: u64,
    epoch_status: EpochStatus,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let account_pubkey = nezha_staking_lib::accounts::latest_epoch(&accounts.nezha_staking_program_id).pubkey;
    let latest_epoch = LatestEpoch {
        account_type: nezha_staking_lib::state::AccountType::LatestEpoch,
        contract_version: nezha_staking_lib::state::ContractVersion::V1,
        is_initialized: true,
        index: epoch_index,
        status: epoch_status,
        epoch: Pubkey::default(),
        cumulative_return_rate: CumulativeReturnRate::unity(),
        pending_funds: Default::default(),
        pubkeys: Default::default(),
    };
    let mut v = Vec::new();
    latest_epoch.serialize(&mut v)?;
    processor.set_account(
        &account_pubkey,
        &Account {
            lamports: 772,
            owner: accounts.nezha_staking_program_id,
            data: v,
        },
    );
    Ok(())
}

// Helpers

pub async fn mint_tokens(
    wallet: &Pubkey,
    amount: u64,
    mint: &Pubkey,
    mint_authority: &Keypair,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let ata = get_associated_token_address(wallet, mint);
    processor
        .send_ixns(
            &[
                spl_token::instruction::mint_to(&spl_token::id(), &mint, &ata, &mint_authority.pubkey(), &[], amount)
                    .unwrap(),
            ],
            &[&mint_authority],
        )
        .await
}

pub async fn create_token_account(owner: &Pubkey, mint: &Pubkey, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let payer = processor.get_payer().pubkey();
    processor
        .send_ixns(
            &[create_associated_token_account(&payer, owner, mint, &spl_token::id())],
            &[],
        )
        .await
}

pub async fn mint_sols(account: &Pubkey, sols: u64, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let payer = processor.get_payer().pubkey();
    processor
        .send_ixns(
            &[system_instruction::transfer(&payer, account, sols * 1_000_000_000)],
            &[],
        )
        .await
}

pub async fn get_vrf_request(
    epoch_index: u64,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<NezhaVrfRequest> {
    get_data(
        ac::nezha_vrf_request(&accounts.program_id, epoch_index).pubkey,
        processor,
    )
    .await
}

// Helper fns

pub async fn get_data<T: BorshDeserialize + HasAccountType>(
    pubkey: Pubkey,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<T> {
    Ok(get_optional_data(pubkey, processor)
        .await
        .with_context(|| format!("Account not found: {pubkey}"))?
        .unwrap())
}

pub async fn get_optional_data<T: BorshDeserialize + HasAccountType>(
    pubkey: Pubkey,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<Option<T>> {
    let account = processor.get_account(pubkey).await?;
    if let Some(account) = account {
        let data = try_from_slice_unchecked(&account.data)
            .map_err(anyhow::Error::from)
            .context(format!(
                "Failed to deserialize data for account: {:?} {}",
                T::account_type(),
                pubkey
            ))?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

pub async fn get_data_packed<T: Pack>(pubkey: Pubkey, processor: &mut dyn SolanaTestRuntime) -> Result<T> {
    let account = processor
        .get_account(pubkey)
        .await?
        .with_context(|| format!("Account not found: {pubkey}"))?;
    Pack::unpack_unchecked(&account.data).with_context(|| format!("Failed to unpack: {pubkey}"))
}

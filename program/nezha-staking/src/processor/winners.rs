//! Winners related processor functions.
use std::ops::DerefMut;

use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::{borsh_deserialize::borsh_deserialize, load_accounts};
use nezha_vrf_lib::accounts as vrf_ac;
use nezha_vrf_lib::state::NezhaVrfRequest;
use solana_program::{
    account_info::AccountInfo, borsh0_10::try_from_slice_unchecked, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::{
    accounts as ac,
    accounts::VerifyPDA,
    error::StakingError,
    fixed_point::FPUSDC,
    instruction::{CreateEpochWinnersMetaArgs, WinnerInput},
    solana,
    state::{
        AccountType, ContractVersion, Epoch, EpochStatus, EpochWinnersMeta, EpochWinnersPage, LatestEpoch, TierStatus,
        TierWinnersMeta, Winner, WinnerProcessingStatus, MAX_NUM_WINNERS_PER_PAGE,
    },
    utils::{check_admin, check_rent_sysvar, check_system_program, check_token_program},
};

#[inline(never)] // This function uses a lot of stack. If inlined, will run out of stack space.
pub fn process_create_epoch_winners_meta<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    meta_args: CreateEpochWinnersMetaArgs,
) -> ProgramResult {
    msg!("Ixn: Create epoch winners meta");
    let account_info_iter = &mut accounts.iter();
    load_accounts!(
        account_info_iter,
        //
        admin_info,
        epoch_winners_meta_info,
        latest_epoch_info,
        epoch_info,
        nezha_vrf_request_info,
        //
        system_program_info,
        rent_info,
    );

    check_system_program(&system_program_info)?;
    check_rent_sysvar(&rent_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let mut latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;
    check_admin(&admin_info, &latest_epoch)?;

    let epoch_index = latest_epoch.index;
    ac::epoch(program_id, epoch_index).verify(&epoch_info)?;
    let mut epoch: Epoch = try_from_slice_unchecked(&mut &epoch_info.data.borrow())?;

    vrf_ac::nezha_vrf_request(&latest_epoch.pubkeys.nezha_vrf_program_id, epoch_index)
        .with_account_type(AccountType::NezhaVrfRequest)
        .verify(nezha_vrf_request_info)?;
    let vrf_request: NezhaVrfRequest = borsh_deserialize(nezha_vrf_request_info)?;

    if latest_epoch.status != EpochStatus::Finalising {
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }

    if epoch.draw_enabled.unwrap_or_default() && vrf_request.winning_combination.is_none() {
        return Err(StakingError::WinningCombinationNotPublished.into());
    }

    let draw_enabled = epoch
        .draw_enabled
        .ok_or_else(|| StakingError::InvalidEpochStatus(latest_epoch.status))?;

    let tier1_meta = TierWinnersMeta {
        total_prize: match meta_args.tier1_meta.total_num_winning_tickets {
            0 => FPUSDC::zero(),
            _ => epoch.yield_split_cfg.jackpot,
        },
        total_num_winners: meta_args.tier1_meta.total_num_winners,
        total_num_winning_tickets: meta_args.tier1_meta.total_num_winning_tickets,
    };
    let tier2_meta = TierWinnersMeta {
        total_prize: match meta_args.tier2_meta.total_num_winning_tickets {
            0 => FPUSDC::zero(),
            _ => latest_epoch.pending_funds.tier2_prize,
        },
        total_num_winners: meta_args.tier2_meta.total_num_winners,
        total_num_winning_tickets: meta_args.tier2_meta.total_num_winning_tickets,
    };
    let tier3_meta = TierWinnersMeta {
        total_prize: match meta_args.tier3_meta.total_num_winning_tickets {
            0 => FPUSDC::zero(),
            _ => latest_epoch.pending_funds.tier3_prize,
        },
        total_num_winners: meta_args.tier3_meta.total_num_winners,
        total_num_winning_tickets: meta_args.tier3_meta.total_num_winning_tickets,
    };

    let total_num_winners = tier1_meta.total_num_winners + tier2_meta.total_num_winners + tier3_meta.total_num_winners;

    // Total number of pages = ceil(total_num_winners / MAX_NUM_WINNERS_PER_PAGE)
    let total_num_pages = total_num_winners
        .saturating_sub(1)
        .checked_div(MAX_NUM_WINNERS_PER_PAGE as u32)
        .ok_or_else(|| StakingError::NumericalOverflow)?
        .checked_add(1)
        .ok_or_else(|| StakingError::NumericalOverflow)?;

    let epoch_winners_meta = if draw_enabled && total_num_winners > 0 {
        EpochWinnersMeta {
            account_type: AccountType::EpochWinnersMeta,
            contract_version: ContractVersion::V1,
            is_initialized: true,
            epoch_pubkey: latest_epoch.epoch,
            epoch_index,
            tier1_meta: tier1_meta.clone(),
            tier2_meta: tier2_meta.clone(),
            tier3_meta: tier3_meta.clone(),
            total_num_pages,
            total_num_winners,
            jackpot_claimable: false,
            status: WinnerProcessingStatus::InProgress {
                num_pages: 0,
                num_processed_winners: 0,
                tier1_status: TierStatus {
                    rem_num_winners: tier1_meta.total_num_winners,
                    rem_num_winning_tickets: tier1_meta.total_num_winning_tickets,
                    rem_prize: tier1_meta.total_prize,
                },
                tier2_status: TierStatus {
                    rem_num_winners: tier2_meta.total_num_winners,
                    rem_num_winning_tickets: tier2_meta.total_num_winning_tickets,
                    rem_prize: tier2_meta.total_prize,
                },
                tier3_status: TierStatus {
                    rem_num_winners: tier3_meta.total_num_winners,
                    rem_num_winning_tickets: tier3_meta.total_num_winning_tickets,
                    rem_prize: tier3_meta.total_prize,
                },
            },
        }
    } else {
        EpochWinnersMeta {
            account_type: AccountType::EpochWinnersMeta,
            contract_version: ContractVersion::V1,
            is_initialized: true,
            epoch_pubkey: latest_epoch.epoch,
            epoch_index,
            tier1_meta: TierWinnersMeta {
                total_prize: FPUSDC::zero(),
                total_num_winners: 0,
                total_num_winning_tickets: 0,
            },
            tier2_meta: TierWinnersMeta {
                total_prize: FPUSDC::zero(),
                total_num_winners: 0,
                total_num_winning_tickets: 0,
            },
            tier3_meta: TierWinnersMeta {
                total_prize: FPUSDC::zero(),
                total_num_winners: 0,
                total_num_winning_tickets: 0,
            },
            total_num_pages: 0,
            total_num_winners: 0,
            jackpot_claimable: false,
            status: WinnerProcessingStatus::Completed,
        }
    };

    let epoch_winners_meta_account = ac::epoch_winners_meta(program_id, epoch_index);
    epoch_winners_meta_account.verify(&epoch_winners_meta_info)?;

    let signer_seeds = epoch_winners_meta_account.seeds();
    if epoch_winners_meta_info.data_is_empty() {
        solana::system_create_account(
            system_program_info,
            epoch_winners_meta_info,
            admin_info,
            rent_info,
            &signer_seeds,
            program_id,
            EpochWinnersMeta::max_len(),
        )?;
    }

    msg!(
        "Meta {} {}",
        epoch_winners_meta.total_num_winners,
        epoch_winners_meta.total_num_pages
    );

    BorshSerialize::serialize(
        &epoch_winners_meta,
        &mut *epoch_winners_meta_info.try_borrow_mut_data()?,
    )?;

    if epoch_winners_meta.status == WinnerProcessingStatus::Completed {
        msg!("No winners to process");
        msg!("Update latest epoch");
        latest_epoch.status = EpochStatus::Ended;
        BorshSerialize::serialize(&latest_epoch, &mut *latest_epoch_info.data.borrow_mut())?;
        msg!("Update epoch");
        epoch.status = EpochStatus::Ended;
        BorshSerialize::serialize(&epoch, &mut *epoch_info.data.borrow_mut())?;
    }

    Ok(())
}

#[inline(never)] // This function uses a lot of stack. If inlined, will run out of stack space.
pub fn process_publish_winners<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    page_index: u32,
    winners_input: Vec<WinnerInput>,
) -> ProgramResult {
    msg!("Ixn: Publish winners");
    let account_info_iter = &mut accounts.iter();
    load_accounts!(
        account_info_iter,
        //
        admin_info,
        latest_epoch_info,
        epoch_info,
        epoch_winners_meta_info,
        epoch_winners_page_info,
        //
        nezha_vrf_request_info,
        system_program_info,
        rent_info,
    );

    check_system_program(&system_program_info)?;
    check_rent_sysvar(&rent_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let mut latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    check_admin(&admin_info, &latest_epoch)?;

    let epoch_index = latest_epoch.index;
    ac::epoch(program_id, epoch_index).verify(&epoch_info)?;
    let mut epoch: Epoch = try_from_slice_unchecked(&epoch_info.data.borrow())?;

    vrf_ac::nezha_vrf_request(&latest_epoch.pubkeys.nezha_vrf_program_id, epoch_index)
        .with_account_type(AccountType::NezhaVrfRequest)
        .verify(nezha_vrf_request_info)?;
    let vrf_request: NezhaVrfRequest = borsh_deserialize(nezha_vrf_request_info)?;

    if latest_epoch.status != EpochStatus::Finalising {
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }
    if vrf_request.winning_combination.is_none() {
        return Err(StakingError::WinningCombinationNotPublished.into());
    }

    ac::epoch_winners_meta(program_id, epoch_index).verify(epoch_winners_meta_info)?;
    let mut epoch_winners_meta_data = epoch_winners_meta_info.try_borrow_mut_data()?;
    let mut epoch_winners_meta = EpochWinnersMeta::try_from_slice(&epoch_winners_meta_data)?;

    if page_index >= epoch_winners_meta.total_num_pages {
        return Err(StakingError::PageIndexOutOfBounds.into());
    }

    if (page_index < epoch_winners_meta.total_num_pages - 1 && winners_input.len() != MAX_NUM_WINNERS_PER_PAGE)
        || winners_input.len() > MAX_NUM_WINNERS_PER_PAGE
    {
        return Err(StakingError::WrongNumberOfWinnersInPage.into());
    }

    let (mut num_pages, mut num_processed_winners, mut tier1_status, mut tier2_status, mut tier3_status) =
        match epoch_winners_meta.status.clone() {
            WinnerProcessingStatus::InProgress {
                num_pages,
                num_processed_winners,
                tier1_status,
                tier2_status,
                tier3_status,
            } => (
                num_pages,
                num_processed_winners,
                tier1_status,
                tier2_status,
                tier3_status,
            ),
            WinnerProcessingStatus::Completed => {
                return Err(StakingError::WinnersAlreadyPublished.into());
            }
        };

    if page_index != num_pages {
        msg!("Page index not in sequence. Expected {}. Got {}", num_pages, page_index);
        return Err(StakingError::PageIndexNotInSequence.into());
    }

    let epoch_winners_page_account = ac::epoch_winners_page(program_id, epoch_index, page_index);
    epoch_winners_page_account.verify(epoch_winners_page_info)?;
    let signer_seeds = epoch_winners_page_account.seeds();

    if epoch_winners_page_info.data_is_empty() {
        solana::system_create_account(
            system_program_info,
            epoch_winners_page_info,
            admin_info,
            rent_info,
            &signer_seeds,
            program_id,
            EpochWinnersPage::max_len(),
        )?;
    }

    let mut epoch_winners_page_data = epoch_winners_page_info.try_borrow_mut_data()?;

    let mut winners = Vec::with_capacity(winners_input.len());

    for (i, winner_input) in winners_input.iter().enumerate() {
        let expected_winner_index = (page_index as usize) * MAX_NUM_WINNERS_PER_PAGE + i;
        if winner_input.index != expected_winner_index as u32 {
            msg!(
                "[{i}]: Expected winner index {}. Got {}",
                expected_winner_index,
                winner_input.index
            );
            return Err(StakingError::UnexpectedWinnerIndex.into());
        }
        if winner_input.index >= epoch_winners_meta.total_num_winners {
            return Err(StakingError::WinnerIndexOutOfBounds.into());
        }

        let tier_meta = match winner_input.tier {
            1 => &epoch_winners_meta.tier1_meta,
            2 => &epoch_winners_meta.tier2_meta,
            3 => &epoch_winners_meta.tier3_meta,
            _ => return Err(StakingError::InvalidWinnerTier.into()),
        };

        let prize = tier_meta
            .total_prize
            .checked_mul(winner_input.num_winning_tickets.into())
            .ok_or_else(|| StakingError::NumericalOverflow)?
            .checked_div(tier_meta.total_num_winning_tickets.into())
            .ok_or_else(|| StakingError::NumericalOverflow)?;

        let winner = Winner {
            index: winner_input.index,
            address: winner_input.address,
            tier: winner_input.tier,
            prize,
            claimed: false,
        };
        let tier_status = match winner.tier {
            1 => &mut tier1_status,
            2 => &mut tier2_status,
            3 => &mut tier3_status,
            _ => return Err(StakingError::InvalidWinnerTier.into()),
        };
        tier_status.rem_num_winning_tickets = tier_status
            .rem_num_winning_tickets
            .checked_sub(winner_input.num_winning_tickets)
            .ok_or_else(|| {
                msg!(
                    "[{i}]: rem_num_winning_tickets {} < winner_input.num_winning_tickets {}",
                    tier_status.rem_num_winning_tickets,
                    winner_input.num_winning_tickets
                );
                StakingError::ProcessedWinnersMetaMismatch
            })?;
        tier_status.rem_num_winners = tier_status.rem_num_winners.checked_sub(1).ok_or_else(|| {
            msg!("[{i}]: rem_num_winners {} < 1", tier_status.rem_num_winners);
            StakingError::ProcessedWinnersMetaMismatch
        })?;
        tier_status.rem_prize = tier_status.rem_prize.checked_sub(prize).ok_or_else(|| {
            msg!(
                "[{i}]: rem_prize {} < prize {}",
                tier_status.rem_prize,
                tier_status.rem_prize
            );
            StakingError::ProcessedWinnersMetaMismatch
        })?;

        winners.push(winner);
        num_processed_winners += 1;
    }
    let page = EpochWinnersPage {
        account_type: AccountType::EpochWinnersPage,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        page_index,
        winners,
    };
    BorshSerialize::serialize(&page, epoch_winners_page_data.deref_mut())?;

    if num_processed_winners > epoch_winners_meta.total_num_winners {
        return Err(StakingError::ProcessedWinnersMetaMismatch.into());
    }

    num_pages += 1;
    if num_pages > epoch_winners_meta.total_num_pages {
        return Err(StakingError::ProcessedWinnersMetaMismatch.into());
    }
    epoch_winners_meta.status = WinnerProcessingStatus::InProgress {
        num_pages,
        num_processed_winners,
        tier1_status: tier1_status.clone(),
        tier2_status: tier2_status.clone(),
        tier3_status: tier3_status.clone(),
    };
    if num_processed_winners == epoch_winners_meta.total_num_winners {
        // We don't check for rem_prize == 0 because of rounding errors
        if num_pages != epoch_winners_meta.total_num_pages
            || tier1_status.rem_num_winners != 0
            || tier1_status.rem_num_winning_tickets != 0
            || tier2_status.rem_num_winners != 0
            || tier2_status.rem_num_winning_tickets != 0
            || tier3_status.rem_num_winners != 0
            || tier3_status.rem_num_winning_tickets != 0
        {
            msg!("Processed winners meta mismatch");
            return Err(StakingError::ProcessedWinnersMetaMismatch.into());
        }
        epoch_winners_meta.status = WinnerProcessingStatus::Completed;
    }
    msg!("Updating winners metadata");
    BorshSerialize::serialize(&epoch_winners_meta, epoch_winners_meta_data.deref_mut())?;

    if let WinnerProcessingStatus::Completed = epoch_winners_meta.status {
        msg!("All winners processed");
        msg!("Update latest epoch");
        if epoch_winners_meta.tier2_meta.total_num_winning_tickets > 0 {
            latest_epoch.pending_funds.tier2_prize = tier2_status.rem_prize;
        }
        if epoch_winners_meta.tier3_meta.total_num_winning_tickets > 0 {
            latest_epoch.pending_funds.tier3_prize = tier3_status.rem_prize;
        }
        msg!("Carry over tier 2 prize {}", latest_epoch.pending_funds.tier2_prize);
        msg!("Carry over tier 3 prize {}", latest_epoch.pending_funds.tier3_prize);
        latest_epoch.status = EpochStatus::Ended;
        BorshSerialize::serialize(&latest_epoch, &mut *latest_epoch_info.data.borrow_mut())?;
        msg!("Update epoch");
        epoch.status = EpochStatus::Ended;
        BorshSerialize::serialize(&epoch, &mut *epoch_info.data.borrow_mut())?;
    }

    Ok(())
}

pub fn process_fund_jackpot<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], epoch_index: u64) -> ProgramResult {
    msg!("Ixn: Fund jackpot");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        funder_info,
        funder_usdc_info,
        epoch_info,
        epoch_winners_meta_info,
        tier1_prize_vault_info,
        token_program_info
    );

    if !funder_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    check_token_program(token_program_info)?;
    ac::epoch(program_id, epoch_index).verify(epoch_info)?;
    ac::epoch_winners_meta(program_id, epoch_index).verify(epoch_winners_meta_info)?;
    ac::prize_vault(program_id, 1).verify(tier1_prize_vault_info)?;

    let mut epoch_winners_meta: EpochWinnersMeta = try_from_slice_unchecked(&epoch_winners_meta_info.data.borrow())?;
    if epoch_winners_meta.jackpot_claimable {
        return Err(StakingError::JackpotAlreadyClaimable.into());
    }

    epoch_winners_meta.jackpot_claimable = true;
    BorshSerialize::serialize(
        &epoch_winners_meta,
        &mut *epoch_winners_meta_info.try_borrow_mut_data()?,
    )?;
    let amount = epoch_winners_meta.tier1_meta.total_prize;

    msg!("Transferring {}", amount);
    solana::token_transfer(
        token_program_info,
        funder_usdc_info,
        tier1_prize_vault_info,
        funder_info,
        None,
        amount.as_usdc(),
    )?;

    Ok(())
}

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::info;
use nezha_vrf_lib::{
    instruction as vrf_instruction,
    state::{NezhaVrfRequest, NezhaVrfRequestStatus},
};
use solana_program::{
    borsh0_10::try_from_slice_unchecked, native_token::LAMPORTS_PER_SOL, program_pack::Pack, pubkey::Pubkey,
};
use solana_sdk::{
    compute_budget,
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

use nezha_staking::{
    accounts as ac,
    fixed_point::FPUSDC,
    francium::constants as fr_consts,
    instruction::{self, CreateEpochWinnersMetaArgs, WinnerInput},
    state::{
        Epoch, EpochWinnersMeta, EpochWinnersPage, LatestEpoch, Stake as SolanaStake, StakeUpdateRequest, TicketsInfo,
        YieldSplitCfg, MAX_NUM_WINNERS_PER_PAGE,
    },
};

use crate::{model::winner::EpochWinners, solana::AccountNotFound};

use super::{
    rpc::{parse_account, SolanaRpc, SolanaRpcExt},
    Stake, SwitchboardDetails, VrfConfiguration, WalletPrize, WithPubkey, WithPubkeyOption,
};
use super::{Solana, SolanaError, ToSolanaError};

#[derive(Clone)]
pub struct SolanaImpl {
    pub rpc_client: Arc<dyn SolanaRpc>,
    pub program_id: Pubkey,
    pub usdc_mint: Pubkey,
    pub nez_mint: Pubkey,
    pub admin_keypair: Arc<Keypair>,
    pub investor_keypair: Arc<Keypair>,
    pub vrf_configuration: VrfConfiguration,
}

#[async_trait]
impl Solana for SolanaImpl {
    async fn get_epoch_vrf_request(&self, epoch_index: u64) -> Result<WithPubkey<NezhaVrfRequest>, SolanaError> {
        let pubkey = nezha_vrf_lib::accounts::nezha_vrf_request(&self.nezha_vrf_program_id(), epoch_index).pubkey;
        let req_acc = match self.rpc_client.get_account(&pubkey).await? {
            Some(acc) => acc,
            None => {
                return Err(SolanaError::AccountNotFound(AccountNotFound::NezhaVrfRequest {
                    program_id: self.nezha_vrf_program_id(),
                    pubkey,
                    epoch_index,
                }))
            }
        };

        // unwrap: this should only be used in test code anyway
        let req: NezhaVrfRequest = try_from_slice_unchecked(&req_acc.data).unwrap();

        Ok(WithPubkey { pubkey, inner: req })
    }
    fn program_id(&self) -> Pubkey {
        self.program_id
    }

    fn admin_keypair(&self) -> Arc<Keypair> {
        self.admin_keypair.clone()
    }

    fn investor_keypair(&self) -> Arc<Keypair> {
        self.investor_keypair.clone()
    }

    fn usdc_mint(&self) -> Pubkey {
        self.usdc_mint
    }

    fn nez_mint(&self) -> Pubkey {
        self.nez_mint
    }

    fn nezha_vrf_program_id(&self) -> Pubkey {
        match self.vrf_configuration {
            VrfConfiguration::Fake { program_id } => program_id.clone(),
            VrfConfiguration::Switchboard { program_id, .. } => program_id.clone(),
        }
    }
    fn vrf_configuration(&self) -> VrfConfiguration {
        self.vrf_configuration.clone()
    }

    // Query Epoch Data
    async fn get_latest_epoch(&self) -> Result<WithPubkey<LatestEpoch>, SolanaError> {
        let latest_epoch_pubkey = ac::latest_epoch(&self.program_id).pubkey;
        let latest_epoch = self
            .rpc_client
            .get_account_parsed::<LatestEpoch>(&latest_epoch_pubkey)
            .await
            .context("Failed to get latest epoch")?;

        match latest_epoch {
            None => Err(SolanaError::AccountNotFound(super::AccountNotFound::LatestEpoch {
                pubkey: latest_epoch_pubkey,
            })),
            Some(latest_epoch) => Ok(latest_epoch),
        }
    }

    async fn get_recent_epochs(&self, n: u64) -> Result<Vec<WithPubkey<Epoch>>, SolanaError> {
        let latest_epoch = self.get_latest_epoch().await?;
        let mut epochs = Vec::with_capacity(n as _);

        let latest_index = latest_epoch.index;
        for i in (latest_index.saturating_sub(n - 1))..=latest_epoch.index {
            match self.get_epoch_by_index(i).await {
                Ok(epoch) => epochs.push(epoch),
                Err(err) => {
                    log::error!("error while getting epoch ({}): {}", i, err);
                    continue;
                }
            }
        }

        Ok(epochs)
    }

    async fn get_epoch_by_index(&self, epoch_index: u64) -> Result<WithPubkey<Epoch>, SolanaError> {
        let addr = ac::epoch(&self.program_id, epoch_index).pubkey;
        self.get_epoch_by_pubkey(addr)
            .await
            .with_context(|| "Failed to get epoch by index")
    }

    async fn get_epoch_by_pubkey(&self, epoch_pubkey: Pubkey) -> Result<WithPubkey<Epoch>, SolanaError> {
        let account = self
            .rpc_client
            .get_account_parsed::<Epoch>(&epoch_pubkey)
            .await
            .with_context(|| "Failed to get epoch by pubkey")?;
        if let Some(account) = account {
            Ok(account)
        } else {
            Err(SolanaError::AccountNotFound(AccountNotFound::Epoch {
                pubkey: epoch_pubkey,
                index: None,
            }))
        }
    }

    async fn get_epoch_winners(&self, epoch_index: u64) -> Result<EpochWinners, SolanaError> {
        let epoch_winners_meta_pubkey = ac::epoch_winners_meta(&self.program_id, epoch_index).pubkey;
        log::info!("get_account_data: EpochDecoder::read_epoch_winners(): EpochWinnersMeta");
        let epoch_winners_meta = self
            .rpc_client
            .get_account_parsed::<EpochWinnersMeta>(&epoch_winners_meta_pubkey)
            .await?
            .ok_or_else(|| {
                SolanaError::AccountNotFound(AccountNotFound::EpochWinnersMeta {
                    epoch_index,
                    pubkey: epoch_winners_meta_pubkey,
                })
            })?;

        let epoch_winner_page_pubkeys = (0..epoch_winners_meta.total_num_pages)
            .map(|page_index| ac::epoch_winners_page(&self.program_id, epoch_index, page_index).pubkey)
            .collect::<Vec<_>>();
        log::info!("get_account_data: EpochDecoder::read_epoch_winners(): EpochWinnersPage");
        let epoch_winner_page_accounts = self
            .rpc_client
            .get_multiple_accounts(&epoch_winner_page_pubkeys)
            .await?;
        let mut winners = Vec::with_capacity(epoch_winners_meta.total_num_winners as usize);
        for (page_index, epoch_winner_page_account) in epoch_winner_page_accounts.into_iter().enumerate() {
            if let Some(epoch_winner_page_account) = epoch_winner_page_account {
                let epoch_winner_page = try_from_slice_unchecked::<EpochWinnersPage>(&epoch_winner_page_account.data)
                    .map_err(|_| {
                    SolanaError::AccountNotFound(AccountNotFound::EpochWinnersPage {
                        epoch_index,
                        page_index: page_index as u32,
                        pubkey: epoch_winner_page_pubkeys[page_index],
                    })
                })?;
                winners.extend_from_slice(&epoch_winner_page.winners);
            } else {
                return Err(SolanaError::AccountNotFound(AccountNotFound::EpochWinnersPage {
                    epoch_index,
                    page_index: page_index as u32,
                    pubkey: epoch_winner_page_pubkeys[page_index],
                }));
            }
        }

        Ok(EpochWinners {
            epoch_index,
            tier1_meta: epoch_winners_meta.tier1_meta.clone(),
            tier2_meta: epoch_winners_meta.tier2_meta.clone(),
            tier3_meta: epoch_winners_meta.tier3_meta.clone(),
            jackpot_claimable: epoch_winners_meta.jackpot_claimable,
            winners,
        })
    }

    async fn request_winning_combination(&self) -> Result<Signature, SolanaError> {
        let SwitchboardDetails {
            switchboard_program_id,
            switchboard_oracle_queue,
            switchboard_oracle_queue_authority,
            switchboard_oracle_queue_mint,
            switchboard_oracle_queue_data_buffer,
        } = self.vrf_configuration.details().unwrap();

        info!(
            "publish winning combination keys: {:?}",
            [
                &self.nezha_vrf_program_id(),
                &self.admin_keypair.pubkey(),
                &switchboard_program_id,
                &switchboard_oracle_queue,
                &switchboard_oracle_queue_authority,
                &switchboard_oracle_queue_mint,
                &switchboard_oracle_queue_data_buffer,
            ]
        );

        let latest_epoch = self.get_latest_epoch().await?;
        self.rpc_client
            .send_and_confirm_transaction(
                &self.admin_keypair,
                &[nezha_vrf_lib::instruction::request_vrf(
                    &self.nezha_vrf_program_id(),
                    &self.admin_keypair.pubkey(),
                    &switchboard_program_id,
                    &switchboard_oracle_queue,
                    &switchboard_oracle_queue_authority,
                    &switchboard_oracle_queue_mint,
                    &switchboard_oracle_queue_data_buffer,
                    &latest_epoch.pubkey,
                    latest_epoch.inner.index,
                )],
            )
            .await
    }

    async fn set_winning_combination_fake(
        &self,
        epoch_index: u64,
        winning_combination: &[u8; 6],
    ) -> Result<Signature, SolanaError> {
        let set_combination = self.check_set_combination(epoch_index).await.unwrap();
        match set_combination {
            Some(combination) if combination.eq(winning_combination) => return Ok(Signature::new_unique()),
            Some(_) => unreachable!("winning combination already set"),
            None => {}
        }

        self.rpc_client
            .send_and_confirm_transaction(
                &self.admin_keypair,
                &[vrf_instruction::mock_set_winning_combination(
                    &self.nezha_vrf_program_id(),
                    &self.admin_keypair.pubkey(),
                    epoch_index,
                    *winning_combination,
                )],
            )
            .await
    }

    // Query User Data

    async fn get_stake_by_wallet(&self, wallet: Pubkey) -> Result<Stake, SolanaError> {
        let latest_epoch_pubkey = ac::latest_epoch(&self.program_id).pubkey;
        let stake_pubkey = ac::stake(&self.program_id, &wallet).pubkey;

        let mut acs = self
            .rpc_client
            .get_multiple_accounts(&[latest_epoch_pubkey, stake_pubkey])
            .await?;

        let stake_ac = acs.pop().unwrap();
        let latest_epoch_ac = acs.pop().unwrap();

        let latest_epoch = match latest_epoch_ac.map(parse_account::<LatestEpoch>).transpose()? {
            None => {
                return Err(SolanaError::AccountNotFound(super::AccountNotFound::LatestEpoch {
                    pubkey: latest_epoch_pubkey,
                }))
            }
            Some(latest_epoch) => latest_epoch,
        };

        let stake = stake_ac.map(parse_account::<SolanaStake>).transpose()?;

        match stake {
            None => Err(SolanaError::AccountNotFound(AccountNotFound::Stake {
                wallet,
                pubkey: stake_pubkey,
            })),
            Some(stake) => Ok(Stake::try_from(stake.into_inner(), &latest_epoch)?),
        }
    }

    async fn get_prizes_by_wallet(&self, wallet: Pubkey) -> Result<Vec<WalletPrize>, SolanaError> {
        let metas = self
            .rpc_client
            .get_program_accounts_by_type_parsed::<EpochWinnersMeta>(&self.program_id)
            .await?;
        let mut prizes = Vec::new();
        for meta in metas {
            let epoch_index = meta.epoch_index;
            let pages = 0..meta.total_num_pages;
            let page_pubkeys = pages
                .clone()
                .map(|page| ac::epoch_winners_page(&self.program_id, epoch_index, page).pubkey)
                .collect::<Vec<_>>();
            let page_accounts = self.rpc_client.get_multiple_accounts(&page_pubkeys).await?;
            for (page, page_account) in pages.zip(page_accounts) {
                if let Some(page_account) = page_account {
                    let page_account =
                        try_from_slice_unchecked::<EpochWinnersPage>(&page_account.data).map_err(|_| {
                            SolanaError::AccountNotFound(AccountNotFound::EpochWinnersPage {
                                epoch_index,
                                page_index: page,
                                pubkey: page_pubkeys[page as usize],
                            })
                        })?;
                    for winner in page_account.winners.iter() {
                        if winner.address == wallet {
                            prizes.push(WalletPrize {
                                epoch_index,
                                page: page as _,
                                winner: winner.clone(),
                            });
                        }
                    }
                }
            }
        }
        Ok(prizes)
    }

    async fn get_stake_update_request_by_wallet(
        &self,
        wallet: Pubkey,
    ) -> Result<Option<StakeUpdateRequest>, SolanaError> {
        let addr = ac::stake_update_request(&self.program_id, &wallet).pubkey;

        let stake_update_request = self.rpc_client.get_account_parsed(&addr).await?;
        Ok(stake_update_request.into_inner())
    }

    async fn get_all_stakes(&self) -> Result<Vec<Stake>, SolanaError> {
        let latest_epoch = self.get_latest_epoch().await?.into_inner();
        let stake_accounts = self
            .rpc_client
            .get_program_accounts_by_type_parsed::<SolanaStake>(&self.program_id)
            .await?;

        let stake_accounts: Vec<Stake> = stake_accounts
            .into_iter()
            .map(
                |stake_account| match Stake::try_from(stake_account.clone(), &latest_epoch) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        log::warn!("Failed to process stake account: {:#?}: {}", stake_account, e);
                        None
                    }
                },
            )
            .flatten()
            .collect();
        Ok(stake_accounts)
    }

    async fn get_all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>, SolanaError> {
        let accs = self
            .rpc_client
            .get_program_accounts_by_type_parsed::<StakeUpdateRequest>(&self.program_id)
            .await?;

        let accs = accs.into_iter().map(WithPubkey::into_inner).collect();
        Ok(accs)
    }

    // Epoch state progression

    async fn create_epoch(
        &self,
        epoch_index: u64,
        expected_end_date: DateTime<Utc>,
        yield_split_cfg: YieldSplitCfg,
    ) -> Result<Signature, SolanaError> {
        let staking_program_id = self.program_id.clone();
        let admin_pubkey = self.admin_keypair.pubkey();

        let instruction = instruction::create_epoch(
            &staking_program_id,
            &admin_pubkey,
            epoch_index,
            expected_end_date.timestamp(),
            yield_split_cfg,
        );

        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[instruction])
            .await?;
        Ok(sig)
    }

    async fn approve_stake_update(&self, wallet: Pubkey, amount: i64) -> Result<Signature, SolanaError> {
        let ix = instruction::approve_stake_update(&self.program_id, &self.admin_keypair.pubkey(), &wallet, amount);

        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[ix])
            .await?;

        Ok(sig)
    }

    async fn complete_stake_update(&self, wallet: Pubkey) -> Result<Signature, SolanaError> {
        let ata = get_associated_token_address(&wallet, &self.usdc_mint);
        let ix = instruction::complete_stake_update(&self.program_id, &self.admin_keypair.pubkey(), &wallet, &ata);

        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[ix])
            .await?;

        Ok(sig)
    }

    async fn enter_investment_fake(&self, epoch_index: u64, num_tickets: u64) -> Result<Signature, SolanaError> {
        let staking_program_id = self.program_id.clone();
        let admin_keypair = self.admin_keypair.clone();
        let admin_pubkey = admin_keypair.pubkey();
        let investor_pubkey = self.investor_keypair.pubkey();
        let investor_usdc_token_pubkey =
            spl_associated_token_account::get_associated_token_address(&investor_pubkey, &self.usdc_mint);

        let instruction = instruction::yield_withdraw_by_investor(
            &staking_program_id,
            &admin_pubkey,
            &investor_usdc_token_pubkey,
            epoch_index,
            TicketsInfo {
                num_tickets,
                tickets_url: String::from("TODO"),
                tickets_hash: Vec::new(),
                tickets_version: 0,
            },
        );

        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[instruction])
            .await?;
        Ok(sig)
    }

    async fn exit_investment_fake(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError> {
        let admin_pubkey = self.admin_keypair.pubkey();
        let investor_pubkey = self.investor_keypair.pubkey();
        let investor_usdc_token_pubkey = get_associated_token_address(&investor_pubkey, &self.usdc_mint);
        let amount = amount.as_usdc();

        let mint_to_ixn = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.usdc_mint,
            &investor_usdc_token_pubkey,
            &admin_pubkey,
            &[&admin_pubkey],
            amount,
        )
        .context("Failed to create MintTo instruction for investor USDC ATA.")?;

        let deposit_ixn = instruction::yield_deposit_by_investor(
            &self.program_id,
            &investor_pubkey,
            &investor_usdc_token_pubkey,
            epoch_index,
            amount,
        );

        let ixs = [mint_to_ixn, deposit_ixn];

        let sig = self
            .rpc_client
            ._send_and_confirm_transaction(
                &[&self.admin_keypair, &self.investor_keypair],
                Some(&self.admin_keypair.pubkey()),
                &ixs,
            )
            .await?;

        Ok(sig)
    }

    async fn enter_investment_francium(&self, epoch_index: u64, num_tickets: u64) -> Result<Signature, SolanaError> {
        let ix = instruction::francium_invest(
            &self.program_id,
            &self.admin_keypair.pubkey(),
            epoch_index,
            TicketsInfo {
                num_tickets,
                tickets_url: String::from("TODO"),
                tickets_hash: Vec::new(),
                tickets_version: 0,
            },
            &fr_consts::get_mints(),
        );
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[ix])
            .await?;
        Ok(sig)
    }

    async fn exit_investment_francium(&self, epoch_index: u64) -> Result<Signature, SolanaError> {
        let cuix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(300000);
        let ix = instruction::francium_withdraw(
            &self.program_id,
            &self.admin_keypair.pubkey(),
            epoch_index,
            &fr_consts::get_mints(),
        );
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[cuix, ix])
            .await?;
        Ok(sig)
    }

    async fn publish_winners(
        &self,
        epoch_index: u64,
        draw_enabled: bool,
        meta_args: &CreateEpochWinnersMetaArgs,
        winners_input: &[WinnerInput],
    ) -> Result<Signature, SolanaError> {
        let cuix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(250_000);
        let create_meta_instruction = instruction::create_epoch_winners_meta(
            &self.program_id,
            &self.admin_keypair.pubkey(),
            epoch_index,
            meta_args.clone(),
            &self.nezha_vrf_program_id(),
        );
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[cuix, create_meta_instruction])
            .await?;

        let total_num_winners = meta_args.tier1_meta.total_num_winners
            + meta_args.tier2_meta.total_num_winners
            + meta_args.tier3_meta.total_num_winners;

        if !draw_enabled || total_num_winners == 0 {
            return Ok(sig);
        }

        let mut last = Signature::default();
        for (page_index, chunk) in winners_input.chunks(MAX_NUM_WINNERS_PER_PAGE).enumerate() {
            let cuix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(700_000);
            let publish_instruction = instruction::publish_winners(
                &self.program_id,
                &self.admin_keypair.pubkey(),
                epoch_index,
                page_index as u32,
                chunk.to_vec(),
                &self.nezha_vrf_program_id(),
            );
            last = self
                .rpc_client
                .send_and_confirm_transaction(&self.admin_keypair, &[cuix, publish_instruction])
                .await?;
        }
        Ok(last)
    }

    async fn fund_jackpot(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError> {
        let admin_pubkey = self.admin_keypair.pubkey();
        let admin_usdc = get_associated_token_address(&admin_pubkey, &self.usdc_mint);

        let mint_to_ixn = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.usdc_mint,
            &admin_usdc,
            &admin_pubkey,
            &[&admin_pubkey],
            amount.as_usdc(),
        )
        .context("Failed to create MintTo instruction")?;

        let fund_ixn = instruction::fund_jackpot(&self.program_id, &admin_pubkey, &admin_usdc, epoch_index);

        let ixns = [mint_to_ixn, fund_ixn];
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &ixns)
            .await?;

        Ok(sig)
    }

    async fn get_usdc_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError> {
        let ata = get_associated_token_address(&wallet, &self.usdc_mint);
        let account = self.rpc_client.get_account(&ata).await?;
        match account {
            Some(account) => {
                let account = spl_token::state::Account::unpack(&account.data)
                    .with_context(|| format!("Failed to decode USDC ATA {} of owner {}", ata, wallet))?;
                Ok(FPUSDC::from_usdc(account.amount))
            }
            None => Ok(FPUSDC::from_usdc(0)),
        }
    }

    async fn create_usdc_ata(&self, wallet: Pubkey) -> Result<Signature, SolanaError> {
        let instruction =
            create_associated_token_account(&self.admin_keypair.pubkey(), &wallet, &self.usdc_mint, &spl_token::id());
        log::info!("Creating USDC ATA for wallet {}", wallet);
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[instruction])
            .await?;
        Ok(sig)
    }

    async fn mint_usdc(&self, wallet: Pubkey, amount: FPUSDC) -> Result<Signature, SolanaError> {
        let admin_pubkey = self.admin_keypair.pubkey();
        let ata_pubkey = get_associated_token_address(&wallet, &self.usdc_mint);
        if let None = self.rpc_client.get_account(&ata_pubkey).await? {
            self.create_usdc_ata(wallet).await?;
        }
        let instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.usdc_mint,
            &ata_pubkey,
            &admin_pubkey,
            &[&admin_pubkey],
            amount.as_usdc(),
        )
        .context("Failed to create mint_to instruction")?;

        // airdrop SOL if admin balance is low
        let account = self.rpc_client.get_account(&admin_pubkey).await?;
        let balance = account.map(|ac| ac.lamports).unwrap_or(0);
        if balance < LAMPORTS_PER_SOL {
            self.rpc_client.request_airdrop(admin_pubkey, LAMPORTS_PER_SOL).await?;
        }

        let transaction_id = self
            .rpc_client
            .send_and_confirm_transaction(&self.admin_keypair, &[instruction])
            .await?;
        Ok(transaction_id)
    }

    async fn get_nez_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError> {
        let ata = get_associated_token_address(&wallet, &self.nez_mint);
        let account = self.rpc_client.get_account(&ata).await?;
        match account {
            Some(account) => {
                let account = spl_token::state::Account::unpack(&account.data)
                    .with_context(|| format!("Failed to decode NEZ ATA {} of owner {}", ata, wallet))?;
                Ok(FPUSDC::from_usdc(account.amount))
            }
            None => Ok(FPUSDC::from_usdc(0)),
        }
    }
}

impl SolanaImpl {
    /// Check if combination can be set, if combination is already set and the status is Success,
    /// the current combination is returned. Otherwise it returns Ok(None) if the combination can
    /// still be set, that is non success status or no combination are set.
    async fn check_set_combination(&self, epoch_index: u64) -> Result<Option<[u8; 6]>, SolanaError> {
        let req = match self.get_epoch_vrf_request(epoch_index).await {
            Ok(req) => req,
            Err(SolanaError::AccountNotFound(_)) => return Ok(None),
            Err(err) => return Err(err),
        };

        if !matches!(req.status, NezhaVrfRequestStatus::Success) {
            return Ok(None);
        }

        Ok(req.winning_combination)
    }
}

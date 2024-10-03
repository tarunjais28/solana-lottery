use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::{error, info, logger};
use nezha_staking::{
    fixed_point::FPUSDC,
    instruction::{CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput},
    state::{EpochStatus, YieldSplitCfg},
};
use solana_program::pubkey::Pubkey;

use crate::{
    model::{
        epoch::{Epoch, EpochError, Investor, UseCache},
        winner::EpochWinners,
    },
    solana::{AccountNotFound, Solana, SolanaError, WalletPrize},
    tickets::{TicketRepository, Winners},
};

use super::{EpochManager, EpochRepository};

pub struct EpochService {
    pub solana: Box<dyn Solana>,
    pub repository: Box<dyn EpochRepository>,
    pub ticket_repository: Box<dyn TicketRepository>,
}

impl EpochService {
    pub fn new(
        solana: Box<dyn Solana>,
        repository: Box<dyn EpochRepository>,
        ticket_repository: Box<dyn TicketRepository>,
    ) -> Self {
        Self {
            solana,
            repository,
            ticket_repository,
        }
    }
}

#[async_trait]
impl EpochManager for EpochService {
    async fn publish_winning_combination(&self, combination: &[u8; 6]) -> Result<Epoch> {
        let latest_epoch = self.solana.get_latest_epoch().await?;

        self.solana
            .set_winning_combination_fake(latest_epoch.index, combination)
            .await
            .map_err(anyhow::Error::from)?;

        self.solana
            .get_epoch_by_index(latest_epoch.index)
            .await
            .map(|e| Epoch::from_solana(&e, Some(*combination)))
            .map_err(anyhow::Error::from)
    }

    async fn request_random_winning_combination(&self) -> Result<Epoch> {
        let latest_epoch = self.solana.get_latest_epoch().await?;

        if self.solana.vrf_configuration().is_fake() {
            bail!("can't use this flow with test switchboard implementation");
        }
        let (combination, status) = match self.solana.get_epoch_vrf_request(latest_epoch.index).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::NezhaVrfRequest { .. })) => {
                info!("Requesting VRF combination");
                self.solana.request_winning_combination().await?;

                return self
                    .solana
                    .get_epoch_by_index(latest_epoch.index)
                    .await
                    .map(|e| Epoch::from_solana(&e, None))
                    .map_err(anyhow::Error::from);
            }
            Err(err) => return Err(anyhow::Error::from(err)),
            Ok(request) => (request.winning_combination, request.status.clone()),
        };
        info!("Combination: {:?}, status: {:?}", combination, status);

        match status {
            nezha_vrf_lib::state::NezhaVrfRequestStatus::Fail => {
                error!("Request status is failed");
            }
            _ => {}
        }

        self.solana
            .get_epoch_by_index(latest_epoch.index)
            .await
            .map(|e| Epoch::from_solana(&e, combination))
            .map_err(anyhow::Error::from)
    }

    async fn epochs(&self) -> Result<Vec<Epoch>> {
        let epochs = self.solana.get_recent_epochs(3).await?;
        let mut res = vec![];

        for epoch in epochs.into_iter() {
            let combination = match self.solana.get_epoch_vrf_request(epoch.index).await {
                Err(SolanaError::AccountNotFound(AccountNotFound::NezhaVrfRequest { .. })) => None,
                Err(err) => return Err(anyhow::Error::from(err)),
                Ok(request) => request.winning_combination,
            };

            res.push(Epoch::from_solana(&epoch, combination))
        }

        Ok(res)
    }

    async fn latest_epoch(&self, use_cache: UseCache) -> Result<Option<Epoch>> {
        log::info!("Fetching latest epoch");
        let latest_epoch_from_cache = match use_cache {
            UseCache::Yes => self.repository.latest_epoch().await?,
            UseCache::No => None,
        };
        if let Some(epoch) = latest_epoch_from_cache {
            return Ok(Some(epoch));
        }

        let latest_epoch = match self.solana.get_latest_epoch().await {
            Err(SolanaError::AccountNotFound(AccountNotFound::LatestEpoch { .. })) => return Ok(None),
            res => res,
        }
        .with_context(|| "Failed to get latest epoch")?;

        if latest_epoch.index == 0 {
            return Ok(None);
        }

        let epoch = self.epoch_by_index(latest_epoch.index, use_cache).await?;

        Ok(epoch)
    }

    async fn epoch_by_index(&self, index: u64, use_cache: UseCache) -> Result<Option<Epoch>> {
        log::info!("Fetching epoch by index: {}", index);
        let epoch_from_cache = match use_cache {
            UseCache::Yes => self.repository.by_index(index).await?,
            UseCache::No => None,
        };
        if let Some(epoch) = epoch_from_cache {
            return Ok(Some(epoch));
        }

        log::info!("Fetching epoch {} from chain", index);
        let epoch = match self.solana.get_epoch_by_index(index).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::Epoch { .. })) => return Ok(None),
            res => res?,
        };

        let combination = match self.solana.get_epoch_vrf_request(epoch.index).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::NezhaVrfRequest { .. })) => None,
            Err(err) => return Err(anyhow::Error::from(err)),
            Ok(request) => request.winning_combination,
        };

        let epoch = Epoch::from_solana(&epoch, combination);
        log::info!("Saving epoch {epoch:#?} to DB");
        self.repository.create_or_update_epoch(&epoch).await?;

        Ok(Some(epoch))
    }

    async fn read_epoch_prizes(&self, index: u64) -> Result<Option<EpochWinners>> {
        let winners = match self.solana.get_epoch_winners(index).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::EpochWinnersMeta { .. }))
            | Err(SolanaError::AccountNotFound(AccountNotFound::EpochWinnersPage { .. })) => return Ok(None),
            res => res?,
        };

        Ok(Some(winners))
    }

    async fn epoch_by_pubkey(&self, pubkey: &Pubkey) -> Result<Option<Epoch>> {
        match self.repository.by_pubkey(pubkey).await? {
            Some(epoch) => return Ok(Some(epoch)),
            _ => {}
        };

        let epoch = match self.solana.get_epoch_by_pubkey(*pubkey).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::Epoch { .. })) => return Ok(None),
            res => res?,
        };

        let combination = match self.solana.get_epoch_vrf_request(epoch.index).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::NezhaVrfRequest { .. })) => None,
            Err(err) => return Err(anyhow::Error::from(err)),
            Ok(request) => request.winning_combination,
        };

        Ok(Some(Epoch::from_solana(&epoch, combination)))
    }

    async fn create_epoch(&self, expected_end_date: DateTime<Utc>, yield_split_cfg: YieldSplitCfg) -> Result<Epoch> {
        let epoch = self.latest_epoch(UseCache::No).await?;
        // If no epoch is found, we need to create the first one
        let epoch_index = match epoch {
            Some(epoch) => epoch.index + 1,
            None => 1,
        };
        log::info!("creating epoch {}", epoch_index);
        self.solana
            .create_epoch(epoch_index, expected_end_date, yield_split_cfg)
            .await?;
        Ok(self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?)
    }

    async fn wallet_prizes(&self, wallet: &Pubkey) -> Result<Vec<WalletPrize>> {
        Ok(self.solana.get_prizes_by_wallet(wallet.clone()).await?)
    }

    async fn enter_investment(&self, investor: Investor) -> Result<Epoch> {
        let epoch = self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?;

        let num_sequences_issued: u64 = self.ticket_repository.num_sequences_by_epoch_index(epoch.index).await?;
        Ok(match epoch.total_invested {
            None => {
                log::info!("Entering investment {:?} for epoch: {}", investor, epoch.index);
                match investor {
                    Investor::Fake => {
                        self.solana
                            .enter_investment_fake(epoch.index, num_sequences_issued)
                            .await?;
                    }
                    Investor::Francium => {
                        self.solana
                            .enter_investment_francium(epoch.index, num_sequences_issued)
                            .await?;
                    }
                }
                self.latest_epoch(UseCache::No)
                    .await?
                    .ok_or(EpochError::CouldNotReadLatestEpoch)?
            }
            Some(_) => epoch,
        })
    }

    async fn exit_investment(&self, investor: Investor, return_amount: Option<FPUSDC>) -> Result<Epoch> {
        let epoch = self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?;
        Ok(match epoch.returns {
            None => {
                log::info!("Exiting investment {:?} for epoch: {}", investor, epoch.index);
                match investor {
                    Investor::Fake => {
                        if let Some(return_amount) = return_amount {
                            self.solana.exit_investment_fake(epoch.index, return_amount).await?;
                        } else {
                            return Err(anyhow!("return amount can't be null"));
                        }
                    }
                    Investor::Francium => {
                        self.solana.exit_investment_francium(epoch.index).await?;
                    }
                }
                self.latest_epoch(UseCache::No)
                    .await?
                    .ok_or(EpochError::CouldNotReadLatestEpoch)?
            }
            Some(_) => epoch,
        })
    }

    async fn publish_winners(&self, winners: Winners) -> Result<Epoch> {
        let latest_epoch = self.solana.get_latest_epoch().await?;
        let epoch_index = latest_epoch.index;
        let epoch = self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?;
        let draw_enabled = epoch
            .draw_enabled
            .ok_or(anyhow!("Publish winners: Draw enabled not set"))?;

        // Already published winners?
        if latest_epoch.status == EpochStatus::Ended {
            return self
                .latest_epoch(UseCache::Yes)
                .await
                .transpose()
                .expect("Will only be None when first epoch hasn't been created yet");
        }

        let mut index = 0;
        let mut winners_input = Vec::new();
        for &address in winners.tier1.iter() {
            winners_input.push(WinnerInput {
                index,
                address,
                tier: 1,
                num_winning_tickets: 1,
            });
            index += 1;
        }
        let tier1_meta = TierWinnersMetaInput {
            total_num_winners: winners.tier1.len() as u32,
            total_num_winning_tickets: winners.tier1.len() as u32,
        };

        let mut tier2_total_num_winning_tickets = 0;
        for (&address, &num_winning_tickets) in winners.tier2.iter() {
            winners_input.push(WinnerInput {
                index,
                address,
                tier: 2,
                num_winning_tickets,
            });
            index += 1;
            tier2_total_num_winning_tickets += num_winning_tickets;
        }
        let tier2_meta = TierWinnersMetaInput {
            total_num_winners: winners.tier2.len() as u32,
            total_num_winning_tickets: tier2_total_num_winning_tickets,
        };

        let mut tier3_total_num_winning_tickets = 0;
        for (&address, &num_winning_tickets) in winners.tier3.iter() {
            winners_input.push(WinnerInput {
                index,
                address,
                tier: 3,
                num_winning_tickets,
            });
            index += 1;
            tier3_total_num_winning_tickets += num_winning_tickets;
        }
        let tier3_meta = TierWinnersMetaInput {
            total_num_winners: winners.tier3.len() as u32,
            total_num_winning_tickets: tier3_total_num_winning_tickets,
        };

        let meta_args = CreateEpochWinnersMetaArgs {
            tier1_meta,
            tier2_meta,
            tier3_meta,
        };

        self.solana
            .publish_winners(epoch_index, draw_enabled, &meta_args, &winners_input)
            .await?;

        Ok(self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?)
    }

    async fn fund_jackpot(&self) -> Result<()> {
        let epoch = self
            .latest_epoch(UseCache::No)
            .await?
            .ok_or(EpochError::CouldNotReadLatestEpoch)?;
        self.solana
            .fund_jackpot(epoch.index, epoch.yield_split_cfg.jackpot)
            .await?;
        Ok(())
    }
}

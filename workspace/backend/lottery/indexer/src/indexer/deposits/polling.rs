use anyhow::Result;
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use crate::nezha_api::{EpochStatus, NezhaAPI, StakeUpdateRequestState};

pub struct AcceptDeposits {
    pub nezha_api: Arc<dyn NezhaAPI + Send + Sync>,
    pub config: AcceptDepositsConfig,
}

pub struct AcceptDepositsConfig {
    pub batch_size: usize,
    pub batch_gap: Duration,
}

impl AcceptDeposits {
    pub async fn run(&self, cancelled: Arc<AtomicBool>) -> Result<()> {
        log::info!("Trying to read pending deposits");
        let deposits = self.nezha_api.all_stake_update_requests().await?;
        log::info!("Received {} deposits pending", deposits.len());

        let mut iter = deposits.chunks(self.config.batch_size);
        while let Some(batch) = iter.next() {
            let start_time = SystemTime::now();
            let epoch_status = self
                .nezha_api
                .get_latest_epoch()
                .await?
                .map(|epoch| epoch.status)
                .unwrap_or(EpochStatus::Ended);

            if cancelled.load(Ordering::Relaxed) {
                return Ok(());
            }
            let mut join_handles = Vec::new();
            for deposit in batch {
                let nezha_api = self.nezha_api.clone();
                let deposit = deposit.clone();
                let handle = tokio::spawn(async move {
                    if deposit.state == StakeUpdateRequestState::PendingApproval {
                        let res = nezha_api.approve_stake_update(&deposit.owner).await;
                        if let Err(e) = res {
                            log::error!("Failed to approve deposits of {}: {}", deposit.owner, e);
                        } else {
                            log::info!("Approved deposit for: {}", deposit.owner);
                        }
                    }

                    if deposit.state == StakeUpdateRequestState::PendingApproval
                        || deposit.state == StakeUpdateRequestState::Queued
                    {
                        if epoch_status != EpochStatus::Running {
                            return;
                        }
                        let res = nezha_api.complete_stake_update(&deposit.owner).await;
                        if let Err(e) = res {
                            log::error!("Failed to e deposits of {}: {}", deposit.owner, e);
                        } else {
                            log::info!("Approved deposit for: {}", deposit.owner);
                        }
                    }
                });
                join_handles.push(handle);
            }
            for handle in join_handles {
                let res = handle.await;
                if let Err(e) = res {
                    log::error!("Failed to join handle: {}. Cause {:?}", e, e.source());
                }
            }
            let elapsed = SystemTime::elapsed(&start_time).unwrap_or_default();
            let remaining_batch_time = self.config.batch_gap.saturating_sub(elapsed);
            tokio::time::sleep(remaining_batch_time).await;
        }
        Ok(())
    }
}

pub struct GenerateTickets {
    pub nezha_api: Box<dyn NezhaAPI + Send + Sync>,
}

impl GenerateTickets {
    pub async fn run(&self) -> Result<()> {
        log::info!("Generating tickets for all");
        let res = self.nezha_api.generate_tickets_for_all().await?;
        log::info!("Generated {} tickets", res.len());
        Ok(())
    }
}

use std::str::FromStr;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use nezha_staking::fixed_point::FPUSDC;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug)]
pub struct Ticket {
    pub wallet: Pubkey,
    pub sequences: Vec<Sequence>,
}

impl Ticket {
    #[cfg(test)]
    pub fn new_for_tests() -> Self {
        Self {
            wallet: Pubkey::new_unique(),
            sequences: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct WalletRisqId {
    pub wallet: Pubkey,
    pub risq_id: String,
}

#[derive(Clone, Debug)]
pub struct Epoch {
    pub index: u64,
    pub pubkey: Pubkey,
    pub prizes: TieredPrizes,
    pub status: EpochStatus,
    pub total_value_locked: Option<FPUSDC>,
    pub winning_combination: Option<[u8; 6]>,
    pub winners: Option<EpochWinners>,
    pub expected_end_date: DateTime<Utc>,
    pub draw_enabled: DrawEnabled,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub enum EpochStatus {
    Running,
    Yielding,
    Finalising,
    Ended,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub enum DrawEnabled {
    NoDraw,
    Waiting,
    Draw,
}

#[derive(Clone, Debug)]
pub struct TieredPrizes {
    pub tier1: String,
    pub tier2_yield_share: u8,
    pub tier3_yield_share: u8,
}

#[derive(Clone, Debug)]
pub struct EpochWinners {
    pub tier1_meta: TierWinnersMeta,
    pub tier2_meta: TierWinnersMeta,
    pub tier3_meta: TierWinnersMeta,
    pub jackpot_claimable: bool,
    pub winners: Vec<Pubkey>,
}

#[derive(Clone, Debug)]
pub struct TierWinnersMeta {
    pub total_num_winners: u32,
    pub total_prize: FPUSDC,
}

#[derive(Clone)]
pub struct StakeUpdateRequest {
    pub owner: Pubkey,
    pub state: StakeUpdateRequestState,
}

#[derive(Clone, PartialEq, Eq)]
pub enum StakeUpdateRequestState {
    PendingApproval,
    Queued,
}

#[derive(Clone, PartialEq, Eq)]
pub enum StakeUpdateType {
    Deposit,
    Withdrawal,
}

#[derive(Clone, Debug)]
pub struct YieldSplitCfg {
    pub insurance_probability: String,
    pub insurance_premium: String,
    pub insurance_jackpot: String,
    pub treasury_ratio: String,
}

#[derive(Clone, Debug)]
pub enum SequenceType {
    Normal,
    SignUpBonus,
    AirdropBonus,
}

#[derive(Clone, Debug)]
pub struct Sequence {
    pub nums: [u8; 6],
    pub sequence_type: SequenceType,
}

#[derive(Copy, Clone, Debug)]
pub enum Investor {
    Francium,
    Fake,
}

impl FromStr for Investor {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "francium" => Ok(Self::Francium),
            "fake" => Ok(Self::Fake),
            _ => Err(anyhow!("Invalid investor")),
        }
    }
}

#[async_trait]
pub trait NezhaAPI {
    async fn get_latest_epoch(&self) -> Result<Option<Epoch>>;
    async fn generate_tickets_for_all(&self) -> Result<Vec<Pubkey>>;
    async fn generate_ticket(&self, wallet: &Pubkey) -> Result<Vec<Sequence>>;
    async fn get_unsubmitted_tickets(&self, epoch_index: u64) -> Result<Vec<Ticket>>;
    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<()>;
    async fn all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>>;
    async fn approve_stake_update(&self, wallet: &Pubkey) -> Result<()>;
    async fn complete_stake_update(&self, wallet: &Pubkey) -> Result<()>;
    async fn create_epoch(
        &self,
        prizes: TieredPrizes,
        expected_duration_minutes: u32,
        yield_split_cfg: YieldSplitCfg,
    ) -> Result<()>;
    async fn enter_investment(&self, investor: Investor) -> Result<()>;
    async fn exit_investment(&self, investor: Investor, return_amount: Option<FPUSDC>) -> Result<()>;
    async fn publish_winners(&self) -> Result<()>;
    async fn calculate_optimal_winning_combination(&self) -> Result<Option<[u8; 6]>>;
    async fn random_winning_combination(&self) -> Result<Option<[u8; 6]>>;
    async fn publish_winning_combination(&self, combination: [u8; 6]) -> Result<()>;
}

pub fn new(url: &str) -> impl NezhaAPI {
    graphql_impl::NezhaAPIImpl::new(url)
}

mod graphql_impl {
    use std::str::FromStr;

    use super::*;
    use anyhow::Context;
    use async_trait::async_trait;
    use graphql_client::GraphQLQuery;
    use log::info;
    use reqwest::Client as HTTPClient;

    type DateTime = chrono::DateTime<chrono::Utc>;
    type WalletAddr = String;
    type TransactionId = String;

    impl TryFrom<TieredPrizes> for create_epoch::PrizesInput {
        type Error = anyhow::Error;
        fn try_from(prizes_input: TieredPrizes) -> Result<Self, Self::Error> {
            Ok(create_epoch::PrizesInput {
                tier1: prizes_input.tier1,
                tier2YieldShare: prizes_input.tier2_yield_share as _,
                tier3YieldShare: prizes_input.tier3_yield_share as _,
            })
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct LatestEpoch;

    impl TryInto<EpochStatus> for latest_epoch::EpochStatus {
        type Error = anyhow::Error;
        fn try_into(self) -> Result<EpochStatus> {
            match self {
                latest_epoch::EpochStatus::RUNNING => Ok(EpochStatus::Running),
                latest_epoch::EpochStatus::YIELDING => Ok(EpochStatus::Yielding),
                latest_epoch::EpochStatus::FINALISING => Ok(EpochStatus::Finalising),
                latest_epoch::EpochStatus::ENDED => Ok(EpochStatus::Ended),
                latest_epoch::EpochStatus::Other(x) => Err(anyhow::anyhow!("Unexpected value for EpochStatus: {}", x)),
            }
        }
    }

    impl TryInto<DrawEnabled> for latest_epoch::DrawEnabled {
        type Error = anyhow::Error;
        fn try_into(self) -> Result<DrawEnabled> {
            match self {
                latest_epoch::DrawEnabled::WAITING => Ok(DrawEnabled::Waiting),
                latest_epoch::DrawEnabled::NO_DRAW => Ok(DrawEnabled::NoDraw),
                latest_epoch::DrawEnabled::DRAW => Ok(DrawEnabled::Draw),
                latest_epoch::DrawEnabled::Other(x) => Err(anyhow::anyhow!("Unexpected value for EpochStatus: {}", x)),
            }
        }
    }

    impl TryFrom<latest_epoch::LatestEpochLatestEpochWinners> for EpochWinners {
        type Error = anyhow::Error;

        fn try_from(epoch_winners: latest_epoch::LatestEpochLatestEpochWinners) -> Result<Self> {
            let mut winners = Vec::with_capacity(epoch_winners.winners.len());
            for winner in epoch_winners.winners {
                winners.push(Pubkey::from_str(&winner.address)?);
            }
            Ok(EpochWinners {
                tier1_meta: TierWinnersMeta {
                    total_num_winners: epoch_winners.tier1_meta.total_num_winners.try_into()?,
                    total_prize: epoch_winners
                        .tier1_meta
                        .total_prize
                        .parse()
                        .map_err(|e: String| anyhow!(e))?,
                },
                tier2_meta: TierWinnersMeta {
                    total_num_winners: epoch_winners.tier2_meta.total_num_winners.try_into()?,
                    total_prize: epoch_winners
                        .tier2_meta
                        .total_prize
                        .parse()
                        .map_err(|e: String| anyhow!(e))?,
                },
                tier3_meta: TierWinnersMeta {
                    total_num_winners: epoch_winners.tier3_meta.total_num_winners.try_into()?,
                    total_prize: epoch_winners
                        .tier3_meta
                        .total_prize
                        .parse()
                        .map_err(|e: String| anyhow!(e))?,
                },
                jackpot_claimable: epoch_winners.jackpot_claimable,
                winners,
            })
        }
    }

    impl TryFrom<latest_epoch::LatestEpochLatestEpoch> for Epoch {
        type Error = anyhow::Error;
        fn try_from(latest_epoch: latest_epoch::LatestEpochLatestEpoch) -> Result<Self> {
            Ok(Self {
                index: latest_epoch.index.try_into()?,
                pubkey: Pubkey::from_str(&latest_epoch.pubkey)?,
                status: latest_epoch.status.try_into()?,
                winners: latest_epoch.winners.map(|x| x.try_into()).transpose()?,
                prizes: TieredPrizes {
                    tier1: latest_epoch.prizes.tier1,
                    tier2_yield_share: latest_epoch.prizes.tier2_yield_share.try_into()?,
                    tier3_yield_share: latest_epoch.prizes.tier3_yield_share.try_into()?,
                },
                total_value_locked: latest_epoch
                    .total_value_locked
                    .map(|amount| amount.parse::<FPUSDC>())
                    .transpose()
                    .map_err(|x| anyhow!(x))?,
                winning_combination: match latest_epoch.winning_combination {
                    None => None,
                    Some(sequence) => {
                        if sequence.len() != 6 {
                            return Err(anyhow::anyhow!(
                                "Invalid winning combination length: {}",
                                sequence.len()
                            ));
                        }
                        let mut winning_combination: [u8; 6] = Default::default();
                        for (i, digit) in sequence.into_iter().enumerate() {
                            winning_combination[i] = digit.try_into()?;
                        }
                        Some(winning_combination)
                    }
                },
                // winners: latest_epoch.winners.map(TryInto::try_into).transpose()?,
                expected_end_date: latest_epoch.expected_end_date.try_into()?,
                draw_enabled: latest_epoch.draw_enabled.try_into()?,
            })
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct UnsubmittedTickets;

    impl TryFrom<unsubmitted_tickets::SequenceType> for SequenceType {
        type Error = anyhow::Error;
        fn try_from(sequence_type: unsubmitted_tickets::SequenceType) -> Result<Self> {
            Ok(match sequence_type {
                unsubmitted_tickets::SequenceType::NORMAL => SequenceType::Normal,
                unsubmitted_tickets::SequenceType::SIGN_UP_BONUS => SequenceType::SignUpBonus,
                unsubmitted_tickets::SequenceType::AIRDROP_BONUS => SequenceType::AirdropBonus,
                unsubmitted_tickets::SequenceType::Other(x) => {
                    Err(anyhow::anyhow!("Unexpected value for SequenceType: {}", x))?
                }
            })
        }
    }

    impl TryFrom<unsubmitted_tickets::UnsubmittedTicketsUnsubmittedTicketsSequences> for Sequence {
        type Error = anyhow::Error;
        fn try_from(sequence: unsubmitted_tickets::UnsubmittedTicketsUnsubmittedTicketsSequences) -> Result<Self> {
            let mut nums: [u8; 6] = Default::default();
            for (i, num) in sequence.nums.into_iter().enumerate() {
                nums[i] = num.try_into()?;
            }
            Ok(Self {
                nums,
                sequence_type: sequence.sequence_type.try_into()?,
            })
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct UpdateRisqIds;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct GenerateTicketsForAll;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct GenerateTicket;

    impl TryFrom<generate_ticket::SequenceType> for SequenceType {
        type Error = anyhow::Error;
        fn try_from(sequence_type: generate_ticket::SequenceType) -> Result<Self> {
            Ok(match sequence_type {
                generate_ticket::SequenceType::NORMAL => SequenceType::Normal,
                generate_ticket::SequenceType::SIGN_UP_BONUS => SequenceType::SignUpBonus,
                generate_ticket::SequenceType::AIRDROP_BONUS => SequenceType::AirdropBonus,
                generate_ticket::SequenceType::Other(x) => {
                    Err(anyhow::anyhow!("Unexpected value for SequenceType: {}", x))?
                }
            })
        }
    }

    impl TryFrom<generate_ticket::GenerateTicketGenerateTicketSequences> for Sequence {
        type Error = anyhow::Error;
        fn try_from(sequence: generate_ticket::GenerateTicketGenerateTicketSequences) -> Result<Self> {
            let mut nums: [u8; 6] = Default::default();
            for (i, num) in sequence.nums.into_iter().enumerate() {
                nums[i] = num.try_into()?;
            }
            Ok(Self {
                nums,
                sequence_type: sequence.sequence_type.try_into()?,
            })
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct AllStakeUpdateRequests;

    impl TryFrom<all_stake_update_requests::StakeUpdateRequestState> for StakeUpdateRequestState {
        type Error = anyhow::Error;
        fn try_from(state: all_stake_update_requests::StakeUpdateRequestState) -> Result<Self> {
            Ok(match state {
                all_stake_update_requests::StakeUpdateRequestState::PENDING_APPROVAL => {
                    StakeUpdateRequestState::PendingApproval
                }
                all_stake_update_requests::StakeUpdateRequestState::QUEUED => StakeUpdateRequestState::Queued,
                all_stake_update_requests::StakeUpdateRequestState::Other(x) => {
                    Err(anyhow::anyhow!("Unexpected value for StakeUpdateState: {}", x))?
                }
            })
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct ApproveStakeUpdate;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct CompleteStakeUpdate;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct CreateEpoch;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct EnterInvestment;

    impl From<Investor> for enter_investment::Investor {
        fn from(investor: Investor) -> Self {
            match investor {
                Investor::Francium => Self::FRACIUM,
                Investor::Fake => Self::FAKE,
            }
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct ExitInvestment;

    impl From<Investor> for exit_investment::Investor {
        fn from(investor: Investor) -> Self {
            match investor {
                Investor::Francium => Self::FRACIUM,
                Investor::Fake => Self::FAKE,
            }
        }
    }

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct PublishWinningCombination;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct PublishWinners;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct CalculateOptimalWinningCombination;

    #[derive(GraphQLQuery)]
    #[graphql(
        schema_path = "src/nezha_api/schema.graphql",
        query_path = "src/nezha_api/query.graphql"
    )]
    struct RandomWinningCombination;

    pub struct NezhaAPIImpl {
        url: String,
        client: HTTPClient,
    }

    impl NezhaAPIImpl {
        pub fn new(url: &str) -> Self {
            Self {
                url: url.to_owned(),
                client: reqwest::Client::new(),
            }
        }
    }

    fn assert_no_errors(query_name: &str, errors: &Option<Vec<graphql_client::Error>>) -> Result<()> {
        if errors.is_none() {
            return Ok(());
        }

        let errors = errors.as_ref().unwrap();

        let error_strings: String = errors
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        Err(anyhow::anyhow!("Error executing GraphQL query {}", query_name).context(error_strings))
    }

    #[async_trait]
    impl NezhaAPI for NezhaAPIImpl {
        async fn get_latest_epoch(&self) -> Result<Option<Epoch>> {
            let res = graphql_client::reqwest::post_graphql::<LatestEpoch, _>(
                &self.client,
                &self.url,
                latest_epoch::Variables {},
            )
            .await?;

            assert_no_errors("get_latest_epoch", &res.errors)?;

            let epoch = res.data.with_context(|| "Response data was null")?.latest_epoch;

            epoch.map(|epoch| epoch.try_into()).transpose()
        }

        async fn generate_tickets_for_all(&self) -> Result<Vec<Pubkey>> {
            let res = graphql_client::reqwest::post_graphql::<GenerateTicketsForAll, _>(
                &self.client,
                &self.url,
                generate_tickets_for_all::Variables {},
            )
            .await?;

            assert_no_errors("generate_tickets_for_all", &res.errors)?;

            Ok(res
                .data
                .unwrap()
                .generate_tickets_for_all
                .iter()
                .map(|x| Pubkey::from_str(&x.wallet).unwrap())
                .collect())
        }

        async fn generate_ticket(&self, wallet: &Pubkey) -> Result<Vec<Sequence>> {
            let res = graphql_client::reqwest::post_graphql::<GenerateTicket, _>(
                &self.client,
                &self.url,
                generate_ticket::Variables {
                    wallet: wallet.to_string(),
                },
            )
            .await?;

            assert_no_errors("generate_ticket", &res.errors)?;

            let sequences = res
                .data
                .ok_or(anyhow!("Response data was null"))?
                .generate_ticket
                .sequences
                .into_iter()
                .map(|x| x.try_into())
                .collect::<Result<Vec<Sequence>>>()?;
            Ok(sequences)
        }

        async fn get_unsubmitted_tickets(&self, epoch_index: u64) -> Result<Vec<Ticket>> {
            let res = graphql_client::reqwest::post_graphql::<UnsubmittedTickets, _>(
                &self.client,
                &self.url,
                unsubmitted_tickets::Variables {
                    epoch_index: epoch_index.try_into()?,
                },
            )
            .await?;

            assert_no_errors("get_unsubmitted_tickets", &res.errors)?;

            let tickets = res.data.with_context(|| "Response data was null")?.unsubmitted_tickets;
            tickets
                .into_iter()
                .map(|ticket| {
                    Ok(Ticket {
                        wallet: Pubkey::from_str(&ticket.wallet).with_context(|| "Can't parse ticket's wallet")?,
                        sequences: ticket
                            .sequences
                            .into_iter()
                            .map(|x| x.try_into())
                            .collect::<Result<Vec<Sequence>>>()?,
                    })
                })
                .collect()
        }

        async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<UpdateRisqIds, _>(
                &self.client,
                &self.url,
                update_risq_ids::Variables {
                    epoch_index: epoch_index.try_into()?,
                    risq_ids: risq_ids
                        .into_iter()
                        .map(|x| update_risq_ids::WalletRisqId {
                            wallet: x.wallet.to_string(),
                            risqId: x.risq_id.clone(),
                        })
                        .collect(),
                },
            )
            .await?;

            assert_no_errors("update_risq_ids", &res.errors)?;

            Ok(())
        }

        async fn all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>> {
            let res = graphql_client::reqwest::post_graphql::<AllStakeUpdateRequests, _>(
                &self.client,
                &self.url,
                all_stake_update_requests::Variables {},
            )
            .await?;

            assert_no_errors("all_stake_update_requests", &res.errors)?;

            let data = res.data.with_context(|| "Response data was null")?;
            let mut stake_update_requests = Vec::with_capacity(data.all_stake_update_requests.len());
            for d in data.all_stake_update_requests {
                stake_update_requests.push(StakeUpdateRequest {
                    owner: Pubkey::from_str(&d.owner).with_context(|| "Can't parse owner pubkey")?,
                    state: d.state.try_into().context("Can't parse stale update request state")?,
                });
            }
            Ok(stake_update_requests)
        }

        async fn approve_stake_update(&self, wallet: &Pubkey) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<ApproveStakeUpdate, _>(
                &self.client,
                &self.url,
                approve_stake_update::Variables {
                    wallet: wallet.to_string(),
                },
            )
            .await?;

            assert_no_errors("approve_stake_update", &res.errors)?;

            Ok(())
        }

        async fn complete_stake_update(&self, wallet: &Pubkey) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<CompleteStakeUpdate, _>(
                &self.client,
                &self.url,
                complete_stake_update::Variables {
                    wallet: wallet.to_string(),
                },
            )
            .await?;

            assert_no_errors("complete_stake_update", &res.errors)?;

            Ok(())
        }

        async fn create_epoch(
            &self,
            prizes: TieredPrizes,
            expected_duration_minutes: u32,
            yield_split_cfg: YieldSplitCfg,
        ) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<CreateEpoch, _>(
                &self.client,
                &self.url,
                create_epoch::Variables {
                    prizes: prizes.try_into()?,
                    expected_duration_minutes: expected_duration_minutes.into(),
                    yield_split_cfg: create_epoch::YieldSplitCfgInput {
                        insurancePremium: yield_split_cfg.insurance_premium,
                        insuranceProbability: yield_split_cfg.insurance_probability,
                        treasuryRatio: yield_split_cfg.treasury_ratio,
                    },
                },
            )
            .await?;

            assert_no_errors("create_epoch", &res.errors)?;

            Ok(())
        }

        async fn enter_investment(&self, investor: Investor) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<EnterInvestment, _>(
                &self.client,
                &self.url,
                enter_investment::Variables {
                    investor: investor.into(),
                },
            )
            .await?;

            assert_no_errors("enter_investment", &res.errors)?;

            Ok(())
        }

        async fn exit_investment(&self, investor: Investor, return_amount: Option<FPUSDC>) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<ExitInvestment, _>(
                &self.client,
                &self.url,
                exit_investment::Variables {
                    investor: investor.into(),
                    return_amount: return_amount.map(|return_amount| return_amount.to_string()),
                },
            )
            .await?;

            assert_no_errors("exit_investment", &res.errors)?;

            Ok(())
        }

        async fn publish_winners(&self) -> Result<()> {
            let res = graphql_client::reqwest::post_graphql::<PublishWinners, _>(
                &self.client,
                &self.url,
                publish_winners::Variables {},
            )
            .await?;

            assert_no_errors("publish_winners", &res.errors)?;

            Ok(())
        }

        async fn calculate_optimal_winning_combination(&self) -> Result<Option<[u8; 6]>> {
            let res = graphql_client::reqwest::post_graphql::<CalculateOptimalWinningCombination, _>(
                &self.client,
                &self.url,
                calculate_optimal_winning_combination::Variables {},
            )
            .await?;

            assert_no_errors("calculate_optimal_winning_combination", &res.errors)?;

            let data = res.data.with_context(|| "Response data was null")?;
            match data.calculate_optimal_winning_combination {
                Some(sequence) => {
                    let winning_combination: [u8; 6] = sequence
                        .into_iter()
                        .map(|n| n.try_into().map_err(|error: <u8 as TryFrom<i64>>::Error| error.into()))
                        .collect::<Result<Vec<u8>>>()?
                        .try_into()
                        .map_err(|_| anyhow!("Cannot convert optimal winning combination to [u8; 6]"))?;
                    Ok(Some(winning_combination))
                }
                None => Ok(None),
            }
        }

        async fn random_winning_combination(&self) -> Result<Option<[u8; 6]>> {
            let res = graphql_client::reqwest::post_graphql::<RandomWinningCombination, _>(
                &self.client,
                &self.url,
                random_winning_combination::Variables {},
            )
            .await?;

            assert_no_errors("random_winning_combination", &res.errors)?;

            let data = res.data.with_context(|| "Response data was null")?;
            Ok(match data.random_winning_combination {
                Some(sequence) => {
                    let winning_combination: [u8; 6] = sequence
                        .into_iter()
                        .map(|n| n.try_into().map_err(|error: <u8 as TryFrom<i64>>::Error| error.into()))
                        .collect::<Result<Vec<u8>>>()?
                        .try_into()
                        .map_err(|_| anyhow!("Cannot convert optimal winning combination to [u8; 6]"))?;
                    Some(winning_combination)
                }
                None => None,
            })
        }

        async fn publish_winning_combination(&self, combination: [u8; 6]) -> Result<()> {
            let winning_combination: Vec<i64> = combination.into_iter().map(|i| i.try_into().unwrap()).collect();
            info!("attempting to publish winning combination {:?}", combination);

            let res = graphql_client::reqwest::post_graphql::<PublishWinningCombination, _>(
                &self.client,
                &self.url,
                publish_winning_combination::Variables { winning_combination },
            )
            .await?;

            assert_no_errors("publish_winning_combination", &res.errors)?;

            Ok(())
        }
    }
}

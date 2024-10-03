use crate::{get_client, Pool};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use postgres_types::ToSql;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::Deserialize;
use service::{
    model::ticket::{TicketError, TicketsWithCount, SEQUENCE_LENGTH},
    tickets::{SequenceType, TicketRepository, WalletRisqId},
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio_postgres::Row;

#[derive(Deserialize, Debug, Clone)]
struct Sequence {
    nums: [i16; SEQUENCE_LENGTH],
    sequence_type: String,
}

impl TryFrom<service::tickets::Sequence> for Sequence {
    type Error = anyhow::Error;
    fn try_from(sequence: service::tickets::Sequence) -> Result<Self> {
        if sequence.nums.len() != SEQUENCE_LENGTH {
            return Err(anyhow!("Invalid sequence length"));
        }
        let mut nums = [0; SEQUENCE_LENGTH];
        for (i, num) in sequence.nums.into_iter().enumerate() {
            nums[i] = num as i16;
        }
        Ok(Self {
            nums,
            sequence_type: sequence.sequence_type.to_string(),
        })
    }
}

impl TryFrom<Sequence> for service::tickets::Sequence {
    type Error = anyhow::Error;
    fn try_from(sequence: Sequence) -> Result<Self> {
        if sequence.nums.len() != SEQUENCE_LENGTH {
            return Err(anyhow!("Invalid sequence length"));
        }
        let mut nums = [0; SEQUENCE_LENGTH];
        for (i, num) in sequence.nums.into_iter().enumerate() {
            nums[i] = u8::try_from(num)?;
        }
        Ok(Self {
            nums,
            sequence_type: SequenceType::from_str(&sequence.sequence_type)?,
        })
    }
}

struct Ticket {
    pub wallet: Pubkey,
    pub epoch_index: u64,
    pub arweave_url: Option<String>,
    pub sequences: Vec<Sequence>,
    pub balance: String,
    pub price: String,
    pub risq_id: Option<String>,
}

impl TryFrom<Row> for Ticket {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Ticket {
            wallet: Pubkey::from_str(&row.get::<_, String>("wallet"))?,
            epoch_index: row
                .get::<_, Decimal>("epoch_index")
                .to_u64()
                .with_context(|| "Can't convert epoch_index to u64")?,
            arweave_url: row.get("arweave_url"),
            sequences: serde_json::from_value(row.get("sequences"))?,
            balance: row.get("balance"),
            price: row.get("price"),
            risq_id: row.get("risq_id"),
        })
    }
}

impl TryFrom<Ticket> for service::tickets::Ticket {
    type Error = anyhow::Error;
    fn try_from(ticket: Ticket) -> Result<Self> {
        Ok(service::tickets::Ticket {
            arweave_url: ticket.arweave_url,
            balance: ticket.balance,
            price: ticket.price,
            epoch_index: ticket.epoch_index,
            sequences: ticket
                .sequences
                .into_iter()
                .map(service::tickets::Sequence::try_from)
                .collect::<Result<Vec<_>>>()?,
            wallet: ticket.wallet,
            risq_id: ticket.risq_id,
        })
    }
}

#[derive(Clone)]
pub struct PostgresTicketRepository {
    pool: Pool,
}

impl PostgresTicketRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TicketRepository for PostgresTicketRepository {
    async fn by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<service::tickets::Ticket>> {
        let client = get_client(&self.pool).await?;

        let row = client
            .query_opt(
                "
                WITH
                    sequences_sel AS (
                        SELECT wallet, epoch_index,
                            to_jsonb(array_agg(jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type))) AS sequences
                        FROM sequences
                        GROUP BY wallet, epoch_index
                    )
                SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id,
                    COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
                FROM ticket
                LEFT JOIN sequences_sel
                ON ticket.wallet=sequences_sel.wallet and ticket.epoch_index=sequences_sel.epoch_index
                WHERE ticket.wallet = $1 AND ticket.epoch_index = $2",
                &[&wallet.to_string(), &Decimal::from(index)],
            ).await?;
        row.map(Ticket::try_from)
            .transpose()?
            .map(service::tickets::Ticket::try_from)
            .transpose()
    }

    async fn by_wallets_and_epoch_index(
        &self,
        wallets: &[Pubkey],
        index: u64,
    ) -> Result<Vec<service::tickets::Ticket>> {
        let client = get_client(&self.pool).await?;

        let wallets: Vec<String> = wallets.iter().map(Pubkey::to_string).collect();
        let rows = client
            .query(
                "
                WITH
                    sequences_sel AS (
                        SELECT wallet, epoch_index,
                            to_jsonb(array_agg(jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type))) AS sequences
                        FROM sequences
                        GROUP BY wallet, epoch_index
                    )
                SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id,
                    COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
                FROM ticket
                LEFT JOIN sequences_sel
                ON ticket.wallet=sequences_sel.wallet and ticket.epoch_index=sequences_sel.epoch_index
                WHERE ticket.wallet = ANY($1) AND ticket.epoch_index = $2",
                &[&wallets, &(Decimal::from(index))],
            )
            .await?;

        rows.into_iter()
            .map(Ticket::try_from)
            .map(|ticket| ticket.map(service::tickets::Ticket::try_from))
            .collect::<Result<Result<Vec<_>>>>()?
    }

    async fn by_epoch_index_and_prefix(
        &self,
        index: u64,
        limit: Option<u8>,
        prefix: &[u8],
    ) -> Result<TicketsWithCount> {
        let client = get_client(&self.pool).await?;
        let prefix_len = prefix.len();
        if prefix_len > SEQUENCE_LENGTH {
            return Err(TicketError::PrefixLengthExceeded(prefix_len).into());
        } else if prefix_len == 0 {
            return Err(TicketError::EmptyPrefix.into());
        }
        let mut query = String::new();
        query += "
        WITH
            sequences_sel AS (
                SELECT wallet, epoch_index,
                    jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type) AS sequences
                FROM sequences
                WHERE epoch_index=$1";
        let epoch_index = Decimal::from(index);
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![&epoch_index];

        let prefix: Vec<i16> = prefix.into_iter().cloned().map(|x| x as i16).collect();
        prefix.iter().enumerate().for_each(|(i, n)| {
            query += &format!(" AND _{} = ${}", i + 1, i + 2);
            params.push(n);
        });

        query += "
            )";

        query += "
        SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id, to_jsonb(array_agg(sequences_sel.sequences)) AS sequences, count(*) OVER() AS count
        FROM ticket
        INNER JOIN sequences_sel
        ON ticket.wallet=sequences_sel.wallet and ticket.epoch_index=sequences_sel.epoch_index
        WHERE ticket.epoch_index=$1
        GROUP BY ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id
        ORDER BY count(sequences) DESC
        ";

        let rows = match limit {
            Some(limit) => {
                query += &format!(" LIMIT ${}", params.len() + 1);
                let limit = limit as i64;
                params.push(&(limit));
                client.query(&query, &params).await?
            }
            None => client.query(&query, &params).await?,
        };

        let count = match rows.get(0) {
            None => 0,
            Some(row) => row.get::<_, i64>("count").try_into()?,
        };
        let tickets = rows
            .into_iter()
            .map(Ticket::try_from)
            .map(|ticket| ticket.map(service::tickets::Ticket::try_from))
            .collect::<Result<Result<Vec<service::tickets::Ticket>>>>()??;
        Ok(TicketsWithCount { tickets, count })
    }

    async fn all(&self) -> Result<Vec<service::tickets::Ticket>> {
        let client = get_client(&self.pool).await?;

        let query = "
        WITH
            sequences_sel AS (
                SELECT wallet, epoch_index,
                    to_jsonb(array_agg(jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type))) AS sequences
                FROM sequences
                GROUP BY wallet, epoch_index
            )
        SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id,
            COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
        FROM ticket
        LEFT JOIN sequences_sel
        ON ticket.wallet=sequences_sel.wallet and ticket.epoch_index=sequences_sel.epoch_index
        "
        .to_string();

        let rows = client.query(&query, &[]).await?;

        rows.into_iter()
            .map(Ticket::try_from)
            .map(|ticket| ticket.map(service::tickets::Ticket::try_from))
            .collect::<Result<Result<Vec<service::tickets::Ticket>>>>()?
    }

    async fn distinct_sequences_by_epoch_index(&self, index: u64) -> Result<Vec<[u8; 6]>> {
        let client = get_client(&self.pool).await?;

        let rows = client
            .query(
                "SELECT DISTINCT ARRAY[_1, _2, _3, _4, _5, _6] AS sequence FROM sequences WHERE epoch_index=$1",
                &[&(Decimal::from(index))],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let seq = row.get::<_, [i16; 6]>("sequence");
                let mut nums = [0u8; 6];
                seq.iter().enumerate().for_each(|(i, n)| {
                    nums[i] = *n as u8;
                });
                nums
            })
            .collect::<Vec<[u8; 6]>>())
    }

    async fn random_sequence_by_epoch_index(&self, index: u64) -> Result<Option<[u8; 6]>> {
        let client = get_client(&self.pool).await?;

        let row = client
            .query_opt(
                "
                SELECT ARRAY[_1, _2, _3, _4, _5, _6] AS sequence
                FROM sequences
                WHERE epoch_index=$1
                ORDER BY random()
                LIMIT 1",
                &[&(Decimal::from(index))],
            )
            .await?;

        Ok(row.map(|row| {
            let seq = row.get::<_, [i16; 6]>("sequence");
            let mut nums = [0u8; 6];
            seq.into_iter().enumerate().for_each(|(i, n)| {
                nums[i] = n as u8;
            });
            nums
        }))
    }

    async fn create(&self, ticket: &service::tickets::Ticket) -> Result<service::tickets::Ticket> {
        let client = get_client(&self.pool).await?;

        let mut query = String::new();
        query += "
        WITH
            ticket_ins AS (
                INSERT INTO ticket (wallet, epoch_index, arweave_url, balance, price, risq_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING *
            ),";
        let wallet = ticket.wallet.to_string();
        let epoch_index = Decimal::from(ticket.epoch_index);
        let sequences = ticket
            .sequences
            .clone()
            .into_iter()
            .map(Sequence::try_from)
            .collect::<Result<Vec<Sequence>>>()?;
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![
            &wallet,
            &epoch_index,
            &ticket.arweave_url,
            &ticket.balance,
            &ticket.price,
            &ticket.risq_id,
        ];

        query += "
            seq (_1, _2, _3, _4, _5, _6, sequence_type) AS (";
        if sequences.is_empty() {
            query += "
                SELECT 1, 2, 3, 4, 5, 6, 'Normal' WHERE false";
        } else {
            query += "
                VALUES ";
            for i in 0..sequences.len() {
                query += &format!(
                    "(${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::varchar)",
                    i * 7 + 7,
                    i * 7 + 8,
                    i * 7 + 9,
                    i * 7 + 10,
                    i * 7 + 11,
                    i * 7 + 12,
                    i * 7 + 13,
                );
                params.push(&sequences[i].nums[0]);
                params.push(&sequences[i].nums[1]);
                params.push(&sequences[i].nums[2]);
                params.push(&sequences[i].nums[3]);
                params.push(&sequences[i].nums[4]);
                params.push(&sequences[i].nums[5]);
                params.push(&sequences[i].sequence_type);
                if i < ticket.sequences.len() - 1 {
                    query += ", ";
                }
            }
        }
        query += "
            ),";
        query += "
            sequences_ins AS (
                INSERT INTO sequences
                SELECT ticket_ins.wallet, ticket_ins.epoch_index, seq._1, seq._2, seq._3, seq._4, seq._5, seq._6, seq.sequence_type
                FROM ticket_ins, seq
                returning *
            ),
            sequences_sel AS (
                SELECT sequences_ins.wallet, sequences_ins.epoch_index,
                    to_jsonb(array_agg(jsonb_build_object('nums', array[sequences_ins._1, sequences_ins._2, sequences_ins._3, sequences_ins._4, sequences_ins._5, sequences_ins._6], 'sequence_type', sequences_ins.sequence_type))) AS sequences
                FROM sequences_ins
                GROUP BY sequences_ins.wallet, sequences_ins.epoch_index
            )
        SELECT ticket_ins.wallet, ticket_ins.epoch_index, ticket_ins.arweave_url, ticket_ins.balance, ticket_ins.price, ticket_ins.risq_id,
            COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
        FROM ticket_ins
        LEFT JOIN sequences_sel
        ON ticket_ins.wallet = sequences_sel.wallet AND ticket_ins.epoch_index=sequences_sel.epoch_index;
        ";

        let row = client.query_one(&query, &params).await?;
        service::tickets::Ticket::try_from(Ticket::try_from(row)?)
    }

    async fn add_sequences(
        &self,
        wallet: &Pubkey,
        epoch_index: u64,
        sequences: &[service::tickets::Sequence],
    ) -> Result<service::tickets::Ticket> {
        let client = get_client(&self.pool).await?;

        let mut query = String::new();
        let wallet = wallet.to_string();
        let epoch_index = Decimal::from(epoch_index);
        let sequences = sequences
            .iter()
            .cloned()
            .map(Sequence::try_from)
            .collect::<Result<Vec<_>>>()?;
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![&wallet, &epoch_index];
        query += "
        WITH
            sequences_ins AS (
                INSERT INTO sequences";
        if sequences.is_empty() {
            query += "
                SELECT $1, $2, 1, 2, 3, 4, 5, 6, 'Normal' WHERE false";
        } else {
            query += "
                VALUES ";
            for i in 0..sequences.len() {
                query += &format!(
                    "($1, $2, ${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::smallint, ${}::varchar)",
                    i * 7 + 3,
                    i * 7 + 4,
                    i * 7 + 5,
                    i * 7 + 6,
                    i * 7 + 7,
                    i * 7 + 8,
                    i * 7 + 9,
                );
                params.push(&sequences[i].nums[0]);
                params.push(&sequences[i].nums[1]);
                params.push(&sequences[i].nums[2]);
                params.push(&sequences[i].nums[3]);
                params.push(&sequences[i].nums[4]);
                params.push(&sequences[i].nums[5]);
                params.push(&sequences[i].sequence_type);
                if i < sequences.len() - 1 {
                    query += ", ";
                }
            }
        }
        query += "
                RETURNING *
            ),";

        query += "
            sequences_sel AS (
                SELECT sequences.wallet, sequences.epoch_index, jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type) AS sequences
                FROM sequences
                UNION
                SELECT sequences_ins.wallet, sequences_ins.epoch_index, jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type) AS sequences
                FROM sequences_ins
            ),
            sequences_agg AS (
                SELECT sequences_sel.wallet, sequences_sel.epoch_index, to_jsonb(array_agg(sequences_sel.sequences)) AS sequences
                FROM sequences_sel
                GROUP BY sequences_sel.wallet, sequences_sel.epoch_index
            )
        SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id,
            COALESCE(sequences_agg.sequences, jsonb_build_array()) AS sequences
        FROM ticket
        LEFT JOIN sequences_agg
        ON ticket.wallet = sequences_agg.wallet AND ticket.epoch_index=sequences_agg.epoch_index
        WHERE ticket.wallet = $1 AND ticket.epoch_index = $2";

        let row = client.query_one(&query, &params).await?;
        let postgres_ticket: Ticket = row.try_into()?;

        Ok(postgres_ticket.try_into()?)
    }

    async fn update_arweave_url(&self, wallet: &Pubkey, index: u64, arweave_url: String) -> Result<Option<()>> {
        let client = get_client(&self.pool).await?;

        let row = client
            .execute(
                "UPDATE ticket SET arweave_url = $1 WHERE wallet = $2 AND epoch_index = $3 AND arweave_url IS NULL",
                &[&arweave_url, &wallet.to_string(), &(Decimal::from(index))],
            )
            .await?;

        Ok((row > 0).then(|| ()))
    }

    async fn get_unsubmitted_tickets_in_epoch(&self, epoch_index: u64) -> Result<Vec<service::tickets::Ticket>> {
        let client = get_client(&self.pool).await?;

        let rows = client
            .query(
                "
                WITH
                    sequences_sel AS (
                        SELECT wallet, epoch_index,
                            to_jsonb(array_agg(jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type))) AS sequences
                        FROM sequences
                        GROUP BY wallet, epoch_index
                    )
                SELECT ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id,
                    COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
                FROM ticket
                LEFT JOIN sequences_sel
                ON ticket.wallet=sequences_sel.wallet and ticket.epoch_index=sequences_sel.epoch_index
                WHERE ticket.risq_id IS NULL AND ticket.epoch_index = $1",
                &[&(Decimal::from(epoch_index))],
            )
            .await?;

        rows.into_iter()
            .map(Ticket::try_from)
            .map(|ticket| ticket.map(service::tickets::Ticket::try_from))
            .collect::<Result<Result<Vec<service::tickets::Ticket>>>>()?
    }

    async fn update_risq_ids(
        &self,
        epoch_index: u64,
        risq_ids: &[WalletRisqId],
    ) -> Result<Vec<service::tickets::Ticket>> {
        let client = get_client(&self.pool).await?;

        let risq_ids: Vec<_> = risq_ids
            .into_iter()
            .map(|wr| (wr.wallet.to_string(), &wr.risq_id))
            .collect();

        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();

        let epoch_index_decimal = Decimal::from(epoch_index);
        params.push(&epoch_index_decimal);

        let mut values = Vec::new();
        for (wallet, risq_id) in &risq_ids {
            let (wallet_idx, risq_id_idx) = (
                params_push_get_idx(&mut params, wallet),
                params_push_get_idx(&mut params, risq_id),
            );
            values.push(format!("(${wallet_idx}, ${risq_id_idx})"));
        }

        let values = values.join(",");

        let query = format!(
            r#"
            WITH
                risq_upd AS (
                    UPDATE ticket
                    SET risq_id = t.risq_id
                    FROM (VALUES {values}) AS t(wallet, risq_id)
                    WHERE ticket.wallet = t.wallet AND ticket.epoch_index = $1 AND ticket.risq_id IS NULL
                    RETURNING ticket.wallet, ticket.epoch_index, ticket.arweave_url, ticket.balance, ticket.price, ticket.risq_id
                ),
                sequences_sel AS (
                    SELECT wallet, epoch_index,
                        to_jsonb(array_agg(jsonb_build_object('nums', array[_1, _2, _3, _4, _5, _6], 'sequence_type', sequence_type))) AS sequences
                    FROM sequences
                    GROUP BY wallet, epoch_index
                )
            SELECT risq_upd.wallet, risq_upd.epoch_index, risq_upd.arweave_url, risq_upd.balance, risq_upd.price, risq_upd.risq_id,
                COALESCE(sequences_sel.sequences, jsonb_build_array()) AS sequences
            FROM risq_upd
            LEFT JOIN sequences_sel
            ON risq_upd.wallet=sequences_sel.wallet and risq_upd.epoch_index=sequences_sel.epoch_index
        "#
        );

        let rows = client.query(&query, &params).await?;
        rows.into_iter()
            .map(Ticket::try_from)
            .map(|ticket| ticket.map(service::tickets::Ticket::try_from))
            .collect::<Result<Result<Vec<service::tickets::Ticket>>>>()?
    }

    async fn num_sequences_by_epoch_index(&self, epoch_index: u64) -> Result<u64> {
        let client = get_client(&self.pool).await?;

        let row = client
            .query_one(
                "SELECT COUNT(*) FROM sequences WHERE epoch_index = $1",
                &[&(Decimal::from(epoch_index))],
            )
            .await?;

        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    async fn num_airdrop_sequences_by_wallet_and_epoch_index(&self, wallet: &Pubkey, epoch_index: u64) -> Result<u32> {
        let client = get_client(&self.pool).await?;

        let row = client
            .query_opt(
                "SELECT SUM(num_sequences) AS num_sequences
                FROM ticket_airdrop
                WHERE wallet = $1 AND epoch_index = $2
                GROUP BY wallet, epoch_index",
                &[&wallet.to_string(), &(Decimal::from(epoch_index))],
            )
            .await?;

        let count = match row {
            None => 0,
            Some(row) => row
                .get::<_, Decimal>("num_sequences")
                .to_u32()
                .ok_or_else(|| anyhow!("failed to convert Decimal to u32"))?,
        };

        Ok(count as u32)
    }

    async fn prior_sequences_exist_by_wallet(&self, wallet: &Pubkey) -> Result<bool> {
        let client = get_client(&self.pool).await?;

        let row = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM sequences WHERE wallet = $1)",
                &[&wallet.to_string()],
            )
            .await?;

        Ok(row.get("exists"))
    }

    // This is using the assumption that a wallet participates in an epoch if it has a sequence in that epoch
    async fn draws_played_by_wallet(&self, wallet: &Pubkey) -> Result<u64> {
        let client = get_client(&self.pool).await?;

        let query = format!(
            r#"
        WITH
            sequences_sel AS (
                SELECT epoch_index
                FROM sequences
                WHERE wallet=$1
                GROUP BY wallet, epoch_index
            )
        SELECT COUNT(*) FROM sequences_sel"#
        );

        let row = client.query_one(&query, &[&wallet.to_string()]).await?;

        let count: i64 = row.get(0);
        Ok(count as u64)
    }
}

fn params_push_get_idx<T>(params: &mut Vec<T>, val: T) -> usize {
    params.push(val);
    params.len()
}

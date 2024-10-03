use anyhow::{Context, Result};

use crate::nezha_api::Ticket;
use risq_api_client::resources::{
    configs::DrawConfig,
    draw::objects::DrawId,
    entry::objects::{Entry, EntryBatch},
};
use solana_sdk::pubkey::Pubkey;
use std::{collections::HashMap, str::FromStr};

pub fn entry_ref_from_ticket(ticket: &Ticket, epoch_index: u64) -> String {
    let wallet = ticket.wallet;
    format!("{wallet}:{epoch_index}")
}

pub fn entry_ref_to_wallet_epoch_index(entry_ref: &str) -> Result<(Pubkey, u64)> {
    let mut split = entry_ref.split(':');
    let wallet = split.next().with_context(|| "Can't extract wallet out of entry_ref")?;
    let epoch_index = split.next().with_context(|| "Can't find epoch index in entry_ref")?;
    let wallet = Pubkey::from_str(wallet).with_context(|| "Can't parse wallet portion of entry_ref")?;
    let epoch_index = u64::from_str(epoch_index).with_context(|| "Can't parse epoch_index portion of entry_ref")?;
    Ok((wallet, epoch_index))
}

pub fn nezha_entry_from_ticket(
    draw_config: &dyn DrawConfig,
    ticket: &Ticket,
    epoch_index: u64,
    draw_id: &DrawId,
    licensee_id: &str,
    timestamp: u64,
) -> Result<Entry> {
    let variant = draw_config.encode_entry_variant(&mut ticket.sequences.iter().map(|e| e.nums.as_ref()))?;
    // Our ID of the player (user in our terminology) that RISQ will store and return to us for
    // cross referencing
    let player_id = ticket.wallet.to_string();
    Ok(Entry {
        draw_id: draw_id.clone(),
        entry_ref: entry_ref_from_ticket(ticket, epoch_index),
        player_id,
        variant,
        timestamp,
        licensee_id: licensee_id.to_string(),
    })
}

pub fn make_nezha_entry_batch_from_tickets(
    draw_config: &dyn DrawConfig,
    batch_id: String,
    tickets: &[Ticket],
    epoch_index: u64,
    draw_id: &DrawId,
    licensee_id: &str,
    timestamp: u64,
) -> EntryBatch {
    let entry_batch = EntryBatch {
        batch_id,
        entries: tickets
            .iter()
            .map(|ticket| {
                let res = nezha_entry_from_ticket(draw_config, ticket, epoch_index, draw_id, licensee_id, timestamp);
                if let Err(x) = &res {
                    log::error!("Failed to generate RISQ Entry for {}: {}", ticket.wallet, x);
                }
                res
            })
            .flatten()
            .collect(),
        signatures: HashMap::new(),
    };
    entry_batch
}

#[cfg(test)]
mod tests {
    use crate::nezha_api::{Sequence, SequenceType, Ticket};
    use anyhow::Result;
    use chrono::Utc;
    use pretty_assertions::assert_eq;
    use risq_api_client::resources::{configs, configs::DrawConfig, entry::objects::Entry};
    use solana_sdk::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_entry_ref_fns() -> Result<()> {
        let t = Ticket {
            wallet: Pubkey::new_unique(),
            sequences: Vec::new(),
        };
        let epoch_index = 12345;
        let entry_ref: String = entry_ref_from_ticket(&t, epoch_index);
        let (wallet_recv, epoch_index_recv) = entry_ref_to_wallet_epoch_index(&entry_ref)?;
        assert_eq!(wallet_recv, t.wallet);
        assert_eq!(epoch_index_recv, epoch_index);
        Ok(())
    }

    #[test]
    fn test_nezha_entry_from_ticket() -> Result<()> {
        let config = configs::nezha::Nezha;
        let epoch_index = 0;
        let wallet = Pubkey::new_unique();
        let ticket = Ticket {
            sequences: vec![
                Sequence {
                    nums: [10, 20, 30, 40, 50, 60],
                    sequence_type: SequenceType::Normal,
                },
                Sequence {
                    nums: [11, 21, 31, 41, 51, 61],
                    sequence_type: SequenceType::Normal,
                },
            ],
            wallet,
        };
        let draw_id = config.next_draw_id(Utc::now().naive_utc().date());
        let licensee_id = "Licensee".to_string();
        let timestamp = 1234;

        let entry_expected = Entry {
            licensee_id: licensee_id.clone(),
            draw_id: draw_id.clone(),
            entry_ref: format!("{wallet}:{epoch_index}"),
            player_id: wallet.to_string(),
            timestamp,
            variant: config.encode_entry_variant(
                &mut [[10, 20, 30, 40, 50, 60], [11, 21, 31, 41, 51, 61]]
                    .iter()
                    .map(|e| e.as_ref()),
            )?,
        };

        let entry = nezha_entry_from_ticket(&config, &ticket, epoch_index, &draw_id, &licensee_id, timestamp)?;

        assert_eq!(entry, entry_expected);
        Ok(())
    }
}

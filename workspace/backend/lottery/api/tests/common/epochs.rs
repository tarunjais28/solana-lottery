use crate::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use service::{
    epoch::EpochManager,
    model::epoch::{Epoch, Prize},
};
use std::sync::RwLock;

#[derive(Default)]
pub struct InMemoryEpochService {
    mem: RwLock<Vec<Epoch>>,
}

impl InMemoryEpochService {
    pub fn add(&mut self, entry: Epoch) {
        self.mem.write().unwrap().push(entry)
    }
}

#[async_trait]
impl EpochManager for InMemoryEpochService {
    async fn read_epochs(&self) -> Result<Vec<Epoch>> {
        Ok(self.mem.read().unwrap().clone())
    }

    async fn read_epoch(&self, index: Option<u64>) -> Result<Option<Epoch>> {
        let epochs_iter = self.mem.read().unwrap();

        let epoch = match index {
            Some(index) => epochs_iter.iter().find(|&v| v.index == index),
            None => epochs_iter.iter().max_by(|&x, &y| x.index.cmp(&y.index)),
        };

        Ok(epoch.map(|e| e.clone()))
    }

    async fn create_epoch(&self) -> Result<Option<Epoch>> {
        let mut mem = self.mem.write().unwrap();
        let index = mem.len() as u64;
        let epoch = Epoch::new(index);
        mem.push(epoch.clone());
        Ok(Some(epoch))
    }

    async fn publish_winners(
        &self,
        index: u64,
        _tier: u8,
        _winning_size: u64,
        _winners: Vec<Pubkey>,
    ) -> Result<Option<Epoch>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .find(|&v| v.index == index)
            .map(|v| v.clone()))
    }

    async fn wallet_prizes(&self, _wallet: &Pubkey) -> Result<Vec<(Pubkey, Prize)>> {
        todo!()
    }

    async fn read_epoch_by_pubkey(&self, epoch: &Pubkey) -> Result<Option<Epoch>> {
        todo!()
    }

    async fn withdraw_yield(&self, _index: u64) -> Result<Option<Epoch>> {
        todo!()
    }
}

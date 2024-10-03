use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ArtkaiUpdater {
    async fn finish_epoch(&self, epoch_index: u64) -> Result<()>;
}

pub struct ArtkaiClient {
    pub client: reqwest::Client,
    pub artkai_webhook_url: String,
    pub artkai_webhook_token: String,
}

impl ArtkaiClient {
    pub fn new(client: reqwest::Client, artkai_webhook_url: String, artkai_webhook_token: String) -> Self {
        ArtkaiClient {
            client,
            artkai_webhook_url,
            artkai_webhook_token,
        }
    }
}

#[async_trait]
impl ArtkaiUpdater for ArtkaiClient {
    async fn finish_epoch(&self, epoch_index: u64) -> Result<()> {
        let res = self
            .client
            .post(&self.artkai_webhook_url)
            .header("x-webhook-token", &self.artkai_webhook_token)
            .json(&serde_json::json!({
                "type": "FINISH_EPOCH",
                "payload": {
                    "index": epoch_index,
                }
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            log::info!("Artkai webhook response: {}", res.text().await?);
        }
        Ok(())
    }
}

pub struct FakeArtkaiClient;

#[async_trait]
impl ArtkaiUpdater for FakeArtkaiClient {
    async fn finish_epoch(&self, _epoch_index: u64) -> Result<()> {
        Ok(())
    }
}

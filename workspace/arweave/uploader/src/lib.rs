pub mod bundlr;
pub mod data;
mod dataitem;
mod deephash;
mod tag;

pub use crate::{
    bundlr::{BundlrClient, BundlrConfig},
    data::DataWithMediaType,
};

use {anyhow::Result, async_trait::async_trait, serde::Serialize};

#[async_trait]
pub trait UploaderClient {
    async fn upload_bytes<T: AsRef<[u8]> + Send>(&self, data_with_media_type: DataWithMediaType<T>) -> Result<String>;
    async fn upload_json<S: Serialize + Sync>(&self, data: &S) -> Result<String>;
}

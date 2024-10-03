use anyhow::Result;
use borsh::BorshDeserialize;
use solana_client::{client_error::ClientErrorKind, nonblocking::rpc_client::RpcClient, rpc_request::RpcError};
use solana_program::pubkey::Pubkey;

pub async fn get_optional_account_data(rpc_client: &RpcClient, pubkey: &Pubkey) -> Result<Option<Vec<u8>>> {
    match rpc_client.get_account_data(&pubkey).await {
        Err(err) => match err.kind {
            ClientErrorKind::RpcError(RpcError::ForUser(err)) if err.contains("AccountNotFound") => Ok(None),
            _ => Err(err.into()),
        },
        Ok(data) => Ok(Some(data)),
    }
}

pub async fn get_optional_account<T: BorshDeserialize>(rpc_client: &RpcClient, pubkey: &Pubkey) -> Result<Option<T>> {
    let data = get_optional_account_data(rpc_client, pubkey).await?;
    Ok(match data {
        None => None,
        Some(data) => Some(T::try_from_slice(&data)?),
    })
}

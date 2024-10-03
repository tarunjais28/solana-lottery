use async_trait::async_trait;
use borsh::BorshDeserialize;
use nezha_staking::state::{AccountType, HasAccountType};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::account::Account as SolanaAccount;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;

use super::{with_pubkey::WithPubkey, SolanaError, ToSolanaError};

mod real;
pub use real::*;

mod test;
pub use test::*;

#[async_trait]
pub trait SolanaRpc: Send + Sync + 'static {
    async fn _send_and_confirm_transaction(
        &self,
        signers: &[&Keypair], // first signer will be the payer
        payer: Option<&Pubkey>,
        instructions: &[Instruction],
    ) -> Result<Signature, SolanaError>;

    async fn get_program_accounts_by_type(
        &self,
        program_id: &Pubkey,
        account_type: AccountType,
    ) -> Result<Vec<WithPubkey<SolanaAccount>>, SolanaError>;

    async fn get_multiple_accounts(
        &self,
        pks: &[Pubkey],
    ) -> Result<Vec<Option<WithPubkey<SolanaAccount>>>, SolanaError>;

    async fn request_airdrop(&self, pubkey: Pubkey, lamports: u64) -> Result<(), SolanaError>;
}

#[async_trait]
pub trait SolanaRpcExt: Send + Sync {
    async fn send_and_confirm_transaction(
        &self,
        signer: &Keypair,
        instructions: &[Instruction],
    ) -> Result<Signature, SolanaError>;

    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<WithPubkey<SolanaAccount>>, SolanaError>;

    async fn get_account_parsed<T: BorshDeserialize + HasAccountType>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<WithPubkey<T>>, SolanaError>;

    async fn get_program_accounts_by_type_parsed<T: BorshDeserialize + HasAccountType>(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<WithPubkey<T>>, SolanaError>;
}

#[async_trait]
impl<U> SolanaRpcExt for U
where
    U: SolanaRpc + ?Sized,
{
    async fn send_and_confirm_transaction(
        &self,
        signer: &Keypair,
        instructions: &[Instruction],
    ) -> Result<Signature, SolanaError> {
        self._send_and_confirm_transaction(&[signer], Some(&signer.pubkey()), instructions)
            .await
    }

    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<WithPubkey<SolanaAccount>>, SolanaError> {
        let mut acs = self
            .get_multiple_accounts(&[*pubkey])
            .await
            .with_context(|| format!("Failed to get account {}", pubkey))?;
        Ok(acs.pop().expect("One element expected because len(pubkeys) = 1"))
    }

    async fn get_account_parsed<T: BorshDeserialize + HasAccountType>(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Option<WithPubkey<T>>, SolanaError> {
        let account = self.get_account(pubkey).await?;
        Ok(match account {
            Some(account) => {
                let acc = parse_account(account)?;
                Some(acc)
            }
            None => None,
        })
    }

    async fn get_program_accounts_by_type_parsed<T: BorshDeserialize + HasAccountType>(
        &self,
        program_id: &Pubkey,
    ) -> Result<Vec<WithPubkey<T>>, SolanaError> {
        let account_type = T::account_type();
        let acs = self.get_program_accounts_by_type(program_id, account_type).await?;

        let mut res = Vec::with_capacity(acs.len());
        for ac in acs {
            match parse_account(ac) {
                Ok(ac_parsed) => res.push(ac_parsed),
                Err(e) => {
                    log::warn!("{}", e);
                }
            }
        }

        Ok(res)
    }
}

pub fn parse_account<T: BorshDeserialize + HasAccountType>(
    account: WithPubkey<SolanaAccount>,
) -> Result<WithPubkey<T>, SolanaError> {
    let t = solana_program::borsh::try_from_slice_unchecked::<T>(&account.data).with_context(|| {
        format!(
            "Failed to parse into {:#?} for account {} ({:?})",
            T::account_type(),
            account.pubkey,
            &account.data[0..5],
        )
    })?;
    Ok(account.replace_inner(t))
}

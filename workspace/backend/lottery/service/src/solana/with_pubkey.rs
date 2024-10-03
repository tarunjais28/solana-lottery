use std::ops::{Deref, DerefMut};

use solana_program::pubkey::Pubkey;

#[derive(Debug)]
pub struct WithPubkey<T> {
    pub inner: T,
    pub pubkey: Pubkey,
}

impl<T> WithPubkey<T> {
    pub fn replace_inner<U>(self, inner: U) -> WithPubkey<U> {
        WithPubkey {
            inner,
            pubkey: self.pubkey,
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

pub trait WithPubkeyOption<T> {
    fn as_inner(&self) -> Option<&T>;
    fn into_inner(self) -> Option<T>;
}

impl<T> WithPubkeyOption<T> for Option<WithPubkey<T>> {
    fn as_inner(&self) -> Option<&T> {
        self.as_ref().map(|x| &x.inner)
    }

    fn into_inner(self) -> Option<T> {
        self.map(|x| x.inner)
    }
}

impl<T> Deref for WithPubkey<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for WithPubkey<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

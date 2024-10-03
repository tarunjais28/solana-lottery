use crate::accounts::AccountType;

/// Used to attach an AccountType value with an account struct.
/// For example, can be used to implement validation logic inside
///     `fn decode<T: AccountType>(data: &[u8])`.
pub trait HasAccountType {
    fn account_type() -> AccountType;
}

#[macro_export]
macro_rules! impl_has_account_type {
    ($state:ty, $account_type:expr) => {
        impl $crate::state::HasAccountType for $state {
            fn account_type() -> AccountType {
                $account_type
            }
        }
    };
}

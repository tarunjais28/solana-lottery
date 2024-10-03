/// This macro helps us write
///     ```
///     accounts![
///         [signer writable] account1,
///         [writable] account2,
///         [] account3,
///     ]
///     ```
///
/// in place of
///     ```
///     vec![
///         AccountMeta::new(account1, true),
///         AccountMeta::new(account2, false),
///         AccountMeta::new_readonly(account2, false),
///     ]
///     ```
#[macro_export]
macro_rules! account_meta {
    ($($modifiers: tt $ac:expr),+$(,)?) => {
        vec![$(account_meta!($modifiers, $ac)),+]
    };
    ([], $ac:expr) => {
        AccountMeta { pubkey: $ac, is_signer: false, is_writable: false }
    };
    ([signer], $ac:expr) => {
        AccountMeta { pubkey: $ac, is_signer: true, is_writable: false }
    };
    ([writable], $ac:expr) => {
        AccountMeta { pubkey: $ac, is_signer: false, is_writable: true }
    };
    ([signer writable], $ac:expr) => {
        AccountMeta { pubkey: $ac, is_signer: true, is_writable: true }
    };
}

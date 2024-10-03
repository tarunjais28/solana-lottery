#[macro_export]
macro_rules! load_accounts {
    ($iter:ident, $($i:ident),+$(,)?) => {
        $(let $i = solana_program::account_info::next_account_info($iter)?;)+
    };
}

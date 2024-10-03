#[macro_export]
macro_rules! impl_borsh_length {
    ($t:ty, $v:literal) => {
        impl $crate::borsh_length::BorshLength for $t {
            fn borsh_length() -> usize {
                $v
            }
        }
    };
}

#[macro_export]
macro_rules! impl_borsh_length_struct {
    ($t:ty, $($field_type:ty),*) => {
        impl $crate::borsh_length::BorshLength for $t {
            fn borsh_length() -> usize {
                $(<$field_type>::borsh_length() + )* 0
            }
        }
    };
}

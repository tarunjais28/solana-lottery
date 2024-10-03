use super::BorshLength;

impl<T: BorshLength> BorshLength for Option<T> {
    fn borsh_length() -> usize {
        1 + T::borsh_length()
    }
}

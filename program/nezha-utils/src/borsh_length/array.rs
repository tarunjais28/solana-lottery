use super::BorshLength;

impl<T: BorshLength, const N: usize> BorshLength for [T; N] {
    fn borsh_length() -> usize {
        T::borsh_length() * N
    }
}

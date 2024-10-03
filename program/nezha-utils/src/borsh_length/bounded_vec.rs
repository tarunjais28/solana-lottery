use super::BorshLength;

pub struct BoundedVec<T: BorshLength, const N: usize>(pub Vec<T>);

impl<T: BorshLength, const N: usize> BorshLength for BoundedVec<T, N> {
    fn borsh_length() -> usize {
        u32::borsh_length() + // borsh stores length of the vec as u32
        T::borsh_length() * N
    }
}

impl<T: BorshLength, const N: usize> std::ops::Deref for BoundedVec<T, N> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: BorshLength, const N: usize> std::ops::DerefMut for BoundedVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

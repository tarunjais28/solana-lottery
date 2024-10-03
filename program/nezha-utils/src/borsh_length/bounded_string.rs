use super::BorshLength;

pub struct BoundedString<const N: usize>(pub String);

impl<const N: usize> BorshLength for BoundedString<N> {
    fn borsh_length() -> usize {
        N
    }
}

impl<const N: usize> std::ops::Deref for BoundedString<N> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> std::ops::DerefMut for BoundedString<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

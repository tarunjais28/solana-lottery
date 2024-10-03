use std::ops::Deref;

// Currently all our PDAs have atmost 5 seed components.
// + 1 for the bump seed.
// If you get assertion error in test after adding a new PDA
// with more seed components, increment this number.
pub const SMALL_VEC_MAX_LEN: usize = 6;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SmallVec<T>(usize, [T; SMALL_VEC_MAX_LEN]);

impl<T: Default + Copy> SmallVec<T> {
    pub fn new() -> Self {
        SmallVec(0, [T::default(); SMALL_VEC_MAX_LEN])
    }
}

impl<T: Copy> SmallVec<T> {
    /// Note: This returns an *empty* SmallVec where
    /// unused slots are filled with the given value
    pub fn new_with_sentinel(i: T) -> Self {
        SmallVec(0, [i; SMALL_VEC_MAX_LEN])
    }
}

impl<T> SmallVec<T> {
    pub fn push(&mut self, x: T) {
        let len = &mut self.0;
        // using debug_assert is safe here because all PDAs
        // defined by our code will be tested in
        //  accounts::tests::guard_against_accidental_changes()
        debug_assert!(*len < SMALL_VEC_MAX_LEN);

        let idx = *len;
        self.1[idx] = x;

        *len += 1;
    }

    pub fn extend_from_iter(&mut self, iter: impl Iterator<Item = T>) {
        for x in iter {
            self.push(x);
        }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.1[0..self.0]
    }
}

impl<T> Deref for SmallVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[test]
fn test_small_vec_as_slice() {
    let a: &[&[u8]] = &["HELLO".as_ref(), "WORLD".as_ref()];
    let mut b: SmallVec<&[u8]> = SmallVec::new_with_sentinel(&[0]);
    b.push("HELLO".as_ref());
    b.push("WORLD".as_ref());

    assert_eq!(a, &*b);
}

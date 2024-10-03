use rand::{seq::SliceRandom, Rng};

pub struct SequenceGenerator<T: Rng> {
    rng: T,
}

impl<T: Rng> SequenceGenerator<T> {
    pub fn new(rng: T) -> Self {
        Self { rng }
    }

    pub fn generate_sequence(&mut self) -> [u8; 6] {
        let mut sequence = [0u8; 6];
        sequence[..5].clone_from_slice(
            &(1..=56)
                .collect::<Vec<u8>>()
                .choose_multiple(&mut self.rng, 5)
                .cloned()
                .collect::<Vec<u8>>(),
        );
        sequence[5] = self.rng.gen_range(1..=10);
        sequence
    }
}

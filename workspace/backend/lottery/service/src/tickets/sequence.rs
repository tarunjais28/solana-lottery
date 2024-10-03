use std::{collections::HashSet, fmt::Display, str::FromStr, sync::Mutex};

use anyhow::{anyhow, Result};
use rand::{prelude::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SequenceType {
    Normal,
    SignUpBonus,
    AirdropBonus,
}

impl Display for SequenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SequenceType::Normal => write!(f, "Normal"),
            SequenceType::SignUpBonus => write!(f, "SignUpBonus"),
            SequenceType::AirdropBonus => write!(f, "AirdropBonus"),
        }
    }
}

impl FromStr for SequenceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Normal" => Ok(SequenceType::Normal),
            "SignUpBonus" => Ok(SequenceType::SignUpBonus),
            "AirdropBonus" => Ok(SequenceType::AirdropBonus),
            _ => Err(anyhow!("Invalid sequence type")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Sequence {
    pub nums: [u8; 6],
    pub sequence_type: SequenceType,
}

pub type Sequences = Vec<Sequence>;

impl From<Sequence> for [u8; 6] {
    fn from(sequence: Sequence) -> Self {
        sequence.nums
    }
}

impl From<Sequence> for [i16; 6] {
    fn from(seq: Sequence) -> Self {
        let mut nums = [0i16; 6];
        for (i, &byte) in seq.nums.iter().enumerate() {
            nums[i] = byte as i16;
        }
        nums
    }
}

pub fn generate_sequences<R: Rng>(
    rng: &Mutex<R>,
    prior: Option<&HashSet<[u8; 6]>>,
    count: u32,
) -> Result<Vec<[u8; 6]>> {
    let mut sequences = match prior {
        None => HashSet::new(),
        Some(prior) => prior.clone(),
    };

    let mut generated = 0u32;
    while generated < count {
        let mut rng = rng.lock().map_err(|e| anyhow!("could not lock RNG mutex: {}", e))?;

        // Choose 5 random bytes out of 56
        let mut sequence_vec = (1..=56)
            .collect::<Vec<u8>>()
            .choose_multiple(&mut *rng, 5)
            .cloned()
            .collect::<Vec<u8>>();

        // Pick 1 random byte out of 10
        let last_number: u8 = rng.gen_range(1..=10);

        sequence_vec.push(last_number);

        let sequence_array: [u8; 6] = sequence_vec
            .try_into()
            .expect("Must not fail, because we designed it to have length=6");
        if sequences.insert(sequence_array) {
            generated += 1;
        }
    }
    match &prior {
        None => Ok(sequences.into_iter().collect()),
        Some(prior) => Ok(sequences.difference(&prior).cloned().collect()),
    }
}

pub fn generate_sequences_with_type<R: Rng>(
    rng: &Mutex<R>,
    prior: Option<&HashSet<[u8; 6]>>,
    count: u32,
    sequence_type: SequenceType,
) -> Result<Vec<Sequence>> {
    let sequences = generate_sequences(rng, prior, count)?
        .into_iter()
        .map(|nums| Sequence { nums, sequence_type })
        .collect();
    Ok(sequences)
}

#[cfg(test)]
mod tests {
    mod sequences_generate {
        use super::super::generate_sequences;
        use rand::{prelude::StdRng, RngCore, SeedableRng};
        use std::{collections::HashSet, result::Result as StdResult, sync::Mutex};

        #[test]
        fn works() {
            let rng = Mutex::new(StdRng::from_seed([0u8; 32]));
            let count = 4;
            let expected_sequences = vec![
                [6, 40, 14, 55, 12, 3],
                [20, 26, 41, 12, 27, 2],
                [22, 18, 46, 41, 5, 7],
                [38, 43, 8, 39, 34, 3],
            ];
            let sequences = generate_sequences(&rng, None, count).unwrap();
            let expected_sequences_set = expected_sequences.iter().cloned().collect::<HashSet<_>>();
            let actual_sequences_set = sequences.iter().cloned().collect::<HashSet<_>>();
            assert_eq!(
                expected_sequences_set, actual_sequences_set,
                "Expected sequences: {expected_sequences_set:?}"
            );
        }

        struct DupRng<R: RngCore> {
            duplicate_count: usize,
            duplicate_issued: usize,
            last_val: u32,
            inner: R,
        }

        impl<R: RngCore> DupRng<R> {
            fn new(mut rng: R, count: usize) -> Self {
                Self {
                    duplicate_count: count,
                    duplicate_issued: 0,
                    last_val: rng.next_u32(),
                    inner: rng,
                }
            }
        }

        impl<R: RngCore> RngCore for DupRng<R> {
            fn next_u32(&mut self) -> u32 {
                if self.duplicate_issued >= self.duplicate_count {
                    self.duplicate_issued = 0;
                    self.last_val = self.inner.next_u32();
                } else {
                    self.duplicate_issued += 1;
                }
                self.last_val
            }

            fn next_u64(&mut self) -> u64 {
                self.next_u32() as _
            }

            fn fill_bytes(&mut self, _dest: &mut [u8]) {
                unimplemented!()
            }

            fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> StdResult<(), rand::Error> {
                unimplemented!()
            }
        }

        #[test]
        fn no_duplicates() {
            let rng = StdRng::from_seed([0u8; 32]);
            let dup_rng = Mutex::new(DupRng::new(rng, 100));
            let count = 4;
            let sequences = generate_sequences(&dup_rng, None, count).unwrap();
            for i in 0..sequences.len() {
                for j in i + 1..sequences.len() {
                    assert_ne!(sequences[i], sequences[j]);
                }
            }
        }
    }
}

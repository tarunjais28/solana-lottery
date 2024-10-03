#[macro_use]
mod macros;

mod primitives;

mod array;
mod option;

mod bounded_vec;
pub use bounded_vec::*;

mod bounded_string;
pub use bounded_string::*;

mod solana;

pub trait BorshLength {
    fn borsh_length() -> usize;
}

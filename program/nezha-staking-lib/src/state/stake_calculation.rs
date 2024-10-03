//! Simplified version of the stake balance calculation logic.
//!
//! We use [`CumulativeReturnRate`](super::CumulativeReturnRate) in combination with
//! [`Stake`](super::Stake) to calculate the stake balances without having to update the stake
//! account of all users each epoch.
//!
//! Idea:
//! ```
//!     // We are using floats here for simplicity.
//!     // Actual impl uses fixed point U192 calculations.
//!     macro_rules! assert_float_eq {
//!         ($a:expr, $b:expr) => {
//!             assert!(($a - $b).abs() < 0.00000001, "{} != {}", $a, $b)
//!         };
//!     }
//!
//!     struct Stake {
//!         amount: f64,
//!         starting_rate: f64,
//!     }
//!
//!     impl Stake {
//!         fn new() -> Stake {
//!             Stake {
//!                 amount: 0.0,
//!                 starting_rate: 1.0,
//!             }
//!         }
//!
//!         fn deposit(&mut self, amount: f64, cumulative_return_rate: f64) {
//!             self.amount = self.balance(cumulative_return_rate);
//!             self.amount += amount;
//!             self.starting_rate = cumulative_return_rate;
//!         }
//!
//!         fn balance(&self, cumulative_return_rate: f64) -> f64 {
//!             self.amount * cumulative_return_rate / self.starting_rate
//!         }
//!     }
//!
//!     let mut cumulative_return_rate: f64 = 1.0;
//!     let mut stake = Stake::new();
//!
//!     // Deposit 100$
//!     stake.deposit(100.0, cumulative_return_rate);
//!     assert_float_eq!(stake.balance(cumulative_return_rate), 100.0);
//!     
//!     // Lose 10%
//!     cumulative_return_rate = cumulative_return_rate * 0.9;
//!     assert_float_eq!(stake.balance(cumulative_return_rate), 90.0);
//!
//!     // Deposit 10$
//!     stake.deposit(10.0, cumulative_return_rate);
//!     assert_float_eq!(stake.balance(cumulative_return_rate), 100.0);
//!
//!     // Gain 10%
//!     cumulative_return_rate = cumulative_return_rate * 1.1;
//!     assert_float_eq!(stake.balance(cumulative_return_rate), 110.0);
//! ```

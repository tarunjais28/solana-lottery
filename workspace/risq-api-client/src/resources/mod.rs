/// We use the following pattern in a bunch of places in draw::objects, entry::objects,
/// and balance::objects.
pub mod balance;
pub mod configs;
pub mod draw;
pub mod entry;

pub use balance::{BalanceRoutes, BalanceRoutesImpl};
pub use draw::{DrawRoutes, DrawRoutesImpl};
pub use entry::{EntryRoutes, EntryRoutesImpl};

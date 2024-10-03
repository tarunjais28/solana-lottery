mod client;

/// We use the following pattern in a bunch of places in `resources::*::objects`.
///
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     common_field: String,
///     #[serde(flatten)]
///     variant: FooVariant
/// }
///
/// #[derive(Serialize, Deserialize)]
/// #[serde(tag = "type")]
/// enum FooVariant {
///     #[serde(rename = "bar")]
///     Bar {
///         bar_field: String,
///     },
///     #[serde(rename = "baz")]
///     Baz {
///         baz_field: String,
///     }
/// }
/// ```
///
/// This will allow us to serialize from/deserialize to a json like this:
/// ```json
/// {
//      type: "bar",
//      common_field: "xyz",
//      bar_field: "asd",
/// }
/// ```
///
/// If you see `#[serde(flatten)]` on a field with an `enum` type, then this pattern is in use
/// there.
pub mod resources;

pub use client::{AppError, AppResult, HttpStatusCode};

use resources::{BalanceRoutes, BalanceRoutesImpl, DrawRoutes, DrawRoutesImpl, EntryRoutes, EntryRoutesImpl};

use crate::client::ReqwestClient;

pub struct RISQClient {
    pub draw: Box<dyn DrawRoutes + Send + Sync>,
    pub entry: Box<dyn EntryRoutes + Send + Sync>,
    pub balance: Box<dyn BalanceRoutes + Send + Sync>,
}

pub fn new_client(base_url: String, partner_id: String, api_key: String) -> RISQClient {
    let client = ReqwestClient::new(base_url, partner_id, api_key);
    RISQClient {
        draw: Box::new(DrawRoutesImpl::new(client.clone())),
        entry: Box::new(EntryRoutesImpl::new(client.clone())),
        balance: Box::new(BalanceRoutesImpl::new(client.clone())),
    }
}

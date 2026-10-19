//! The daemon a run dials, as whom — dialled only when a request is
//! put.

use diverge_sdk::wire::client::handle::Handle;

use super::{Error, dial};

/// The selected daemon's address and the selected account's key: what
/// a leaf dials with, when its command line puts a request and not
/// something answered without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target<'a> {
    /// The daemon's WebSocket address.
    pub address: &'a str,
    /// The account's key.
    pub key: &'a str,
}

impl Target<'_> {
    /// The daemon dialled as the account: see [`dial`].
    pub async fn dial(&self) -> Result<Handle, Error> {
        dial(self.address, self.key).await
    }
}

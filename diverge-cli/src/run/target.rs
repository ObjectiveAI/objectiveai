//! The daemon a run dials, as whom — dialled only when a request is
//! put.

use diverge_sdk::wire::client::dial::{DialError, dial};
use diverge_sdk::wire::client::handle::Handle;

use super::Error;

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
    /// The daemon dialled as the account, with the SDK's [`dial`]: the
    /// router reading on a task of its own, the handle every request
    /// is put on. A socket that could not be made is [`Error::Dial`];
    /// a credential that could not be presented, [`Error::Authorize`].
    pub async fn dial(&self) -> Result<Handle, Error> {
        let (handle, run) = dial(self.address, self.key).await.map_err(|error| match error {
            DialError::Dial(why) => Error::Dial(why),
            DialError::Authorize(why) => Error::Authorize(why),
        })?;
        tokio::spawn(run);
        Ok(handle)
    }
}

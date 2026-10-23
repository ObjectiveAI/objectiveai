//! The daemon a run dials, as whom — dialled only when a request is
//! put.

use std::path::Path;

use diverge_sdk::keyring::Keyring;
use diverge_sdk::wire::client::dial::{DialError, dial};
use diverge_sdk::wire::client::handle::Handle;

use super::Error;

/// The selected daemon's address and the keyring entry of the
/// selected account's key: what a leaf dials with, when its command
/// line puts a request and not something answered without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target<'a> {
    /// The daemon's WebSocket address.
    pub address: &'a str,
    /// The root, whose keyring holds the key.
    pub root: &'a Path,
    /// The entry holding the account's key.
    pub key: &'a str,
}

impl Target<'_> {
    /// The daemon dialled as the account, with the SDK's [`dial`]: the
    /// key read from the keyring, then the router reading on a task of
    /// its own, the handle every request is put on. A keyring that
    /// could not be read is [`Error::Keyring`], an entry it does not
    /// hold [`Error::NoKey`]; a socket that could not be made is
    /// [`Error::Dial`]; a credential that could not be presented,
    /// [`Error::Authorize`].
    pub async fn dial(&self) -> Result<Handle, Error> {
        let keyring = Keyring::open(self.root).await.map_err(|error| Error::Keyring(error.to_string()))?;
        let key = keyring
            .get(self.key)
            .await
            .map_err(|error| Error::Keyring(error.to_string()))?
            .ok_or_else(|| Error::NoKey(self.key.to_string()))?;
        let (handle, run) = dial(self.address, key.expose()).await.map_err(|error| match error {
            DialError::Dial(why) => Error::Dial(why),
            DialError::Authorize(why) => Error::Authorize(why),
        })?;
        tokio::spawn(run);
        Ok(handle)
    }
}

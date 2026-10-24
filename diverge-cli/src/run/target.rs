//! The daemon a run dials, as whom — dialled only when a request is
//! put.

use std::path::Path;

use diverge_sdk::identities::{Identities, Name, Presenter};
use diverge_sdk::wire::client::dial::{DialError, dial};
use diverge_sdk::wire::client::handle::Handle;

use super::Error;
use crate::config::Account;

/// The selected daemon's address and the selected account: what a
/// leaf dials with, when its command line puts a request and not
/// something answered without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target<'a> {
    /// The daemon's WebSocket address.
    pub address: &'a str,
    /// The root, whose identities hold the account's.
    pub root: &'a Path,
    /// The account: the identity the CLI presents.
    pub account: &'a Account,
}

impl Target<'_> {
    /// The daemon dialled as the account, with the SDK's [`dial`]: the
    /// handshake made as its connector by a [`Presenter`] — the
    /// account's access token from its issuer presented, refreshed
    /// where it must be, and the daemon's answer taken at its word,
    /// since the CLI judges no daemon — then the router reading on a
    /// task of its own, the handle every request is put on. Identities
    /// that could not be opened are [`Error::Identities`]; a socket
    /// that could not be made is [`Error::Dial`]; a handshake that did
    /// not complete — the account's token not to be had, or the daemon
    /// closing on it — [`Error::Handshake`].
    pub async fn dial(&self) -> Result<Handle, Error> {
        let identities = Identities::open(self.root)
            .await
            .map_err(|error| Error::Identities(error.to_string()))?;
        let name = Name {
            issuer: self.account.issuer.clone(),
            subject: self.account.subject.clone(),
        };
        let presenter = Presenter::new(identities, name);
        let (handle, run, _) = dial(self.address, &self.account.issuer, &presenter)
            .await
            .map_err(|error| match error {
                DialError::Dial(why) => Error::Dial(why),
                DialError::Handshake(why) => Error::Handshake(why),
            })?;
        tokio::spawn(run);
        Ok(handle)
    }
}

//! What is done through providers, to the daemons connected to them.

use serde::{Deserialize, Serialize};

/// The actions through providers, which a grant reaches as far as its
/// `within` says: the providers whose records the `within` reaches, by
/// their tags. Snake case on the wire: `"list"`, `"listen"`,
/// `"message"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// List the identities their listings hold, as
    /// [`providers::daemons::list`](crate::daemon::endpoints::providers::daemons::list)
    /// does; the listing holds only the routes through the providers
    /// the grant reaches.
    List,
    /// Hear the messages sent to the daemon through them, as
    /// [`providers::daemons::listen`](crate::daemon::endpoints::providers::daemons::listen)
    /// does; the daemon sends only those that came through the
    /// providers the grant reaches.
    Listen,
    /// Send messages through them, as
    /// [`providers::daemons::message`](crate::daemon::endpoints::providers::daemons::message)
    /// does; a message is sent through no provider the grant does not
    /// reach.
    Message,
}

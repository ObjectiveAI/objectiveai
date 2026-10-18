//! One account this daemon holds on another daemon, as a list
//! reports it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::outgoing::Kind;

/// One account on a daemon record: its name, of the caller's
/// choosing, and the kind of its mode without the credential, which
/// the wire never answers. What a connected tool and a relayed
/// request name the daemon by, beside the record's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Account {
    /// The name, as its add gave it: unique among the record's
    /// accounts. Compared and not read.
    pub name: String,
    /// Which mode this daemon authenticates to it in, without the
    /// credential. See [`Kind`].
    pub kind: Kind,
}

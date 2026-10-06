//! Which database the daemon serves: its own, or one it dials.

use serde::{Deserialize, Serialize};

/// Which database the daemon splices container connections onto. On the
/// wire one object naming its kind: `{"kind":"local"}`, or
/// `{"kind":"remote","url":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Mode {
    /// A Postgres the daemon runs itself, beside it, on its own
    /// machine. There is nothing to give: where it is, and as whom the
    /// daemon reaches it, are the daemon's own.
    Local,
    /// A Postgres the daemon dials, wherever the URL says.
    Remote {
        /// Where to dial and as whom, as a Postgres URL —
        /// `postgres://user:password@host:port/database`, with whatever
        /// parameters the daemon's own driver takes. The daemon dials
        /// it anew for every container connection, as that connection's
        /// other end; what it does with the user and the database the
        /// container named is the daemon's. A password in it is kept
        /// and never answered: a get reports the URL with the password
        /// taken out.
        url: String,
    },
}

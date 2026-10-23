//! Which database the daemon runs on, and what its own is given.

use serde::{Deserialize, Serialize};

/// The connections the local cluster accepts when the document does
/// not say.
const MAX_CONNECTIONS: u32 = 1024;

/// The `postgres` of `daemon.yaml`: a Postgres the daemon runs beside
/// itself, with what that cluster is given, or one it dials. Tagged
/// `kind`, as the daemon's own
/// [`Mode`](diverge_sdk::daemon::endpoints::postgres::Mode) on the
/// wire is — `{kind: local, max_connections: 1024}`,
/// `{kind: remote, url: …}` — and, unlike it, refusing a key the kind
/// does not have.
///
/// The daemon reads it for which; in the local kind it starts
/// `diverge-postgres` and hands it the cluster's settings as
/// arguments, so the supervisor reads no document of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Postgres {
    /// A Postgres the daemon runs itself, under `<root>/daemon/postgres/`.
    Local {
        /// The most connections the postmaster accepts at once, passed
        /// to it as `max_connections`. Every container connection the
        /// daemon splices through is one of them, so this is sized for
        /// a daemon with many containers rather than for a desktop.
        /// Absent means a thousand and twenty-four.
        #[serde(default = "max_connections")]
        max_connections: u32,
    },
    /// A Postgres the daemon dials, wherever the URL says.
    Remote {
        /// Where to dial and as whom, as a Postgres URL —
        /// `postgres://user@host:port/database`, with whatever
        /// parameters the daemon's own driver takes, and no password.
        url: String,
        /// The keyring entry the password is kept in; absent, the
        /// daemon dials with none. The daemon sets it into the URL as
        /// it dials, and its `postgres get` reports the URL with the
        /// password taken out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
}

/// The local cluster, with its defaults.
impl Default for Postgres {
    fn default() -> Self {
        Postgres::Local {
            max_connections: MAX_CONNECTIONS,
        }
    }
}

/// What `max_connections` takes when the document leaves it out.
fn max_connections() -> u32 {
    MAX_CONNECTIONS
}

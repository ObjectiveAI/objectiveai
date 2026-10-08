//! Which database the daemon runs on, and what its own is given.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::postgres::Mode;

/// The connections the local cluster accepts when the block does not
/// say.
const MAX_CONNECTIONS: u32 = 1024;

/// The `postgres` of the daemon's block: a Postgres the daemon runs
/// beside itself, with what that cluster is given, or one it dials.
/// Tagged `kind` on the wire, as the daemon's own
/// [`Mode`] is — `{kind: local, max_connections: 1024}`,
/// `{kind: remote, url: …}` — and, unlike it, refusing a key the
/// kind does not have.
///
/// `diverge-daemon` reads it for which; `diverge-postgres`, the
/// supervisor the daemon starts in the local mode, reads it for the
/// cluster's own settings, and refuses to start under the remote
/// kind, where no cluster is wanted.
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
        /// `postgres://user:password@host:port/database`, with whatever
        /// parameters the daemon's own driver takes. A password in it
        /// is kept and never answered: the daemon's `postgres get`
        /// reports the URL with the password taken out.
        url: String,
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

impl Postgres {
    /// The same choice as the wire states it: local, or remote with
    /// the URL.
    pub fn mode(&self) -> Mode {
        match self {
            Postgres::Local { .. } => Mode::Local,
            Postgres::Remote { url } => Mode::Remote { url: url.clone() },
        }
    }
}

/// What `max_connections` takes when the block leaves it out.
fn max_connections() -> u32 {
    MAX_CONNECTIONS
}

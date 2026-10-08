//! Either mode, as one URL and one thing to stop.

use std::path::Path;

use diverge_sdk::config;

use super::{Error, Local};

/// The database the daemon runs on, held until the stop: the program
/// beside it in the local mode, and nothing to hold in the remote.
#[derive(Debug)]
pub enum Postgres {
    /// The daemon's own, running.
    Local(Local),
    /// Somebody else's, dialled.
    Remote,
}

/// Start, or name, the database `postgres` says, for the file at
/// `root`, and hand back what is held and the URL the store opens:
/// the local cluster's, which names no database, or the remote URL
/// exactly as configured.
pub async fn start(postgres: &config::daemon::Postgres, root: &Path) -> Result<(Postgres, String), Error> {
    match postgres {
        config::daemon::Postgres::Local { .. } => {
            let (local, url) = super::spawn(root).await?;
            Ok((Postgres::Local(local), url))
        }
        config::daemon::Postgres::Remote { url } => Ok((Postgres::Remote, url.clone())),
    }
}

impl Postgres {
    /// Whether the database is the daemon's own, which decides whether
    /// the store names the database inside it or takes the URL whole.
    pub fn is_local(&self) -> bool {
        matches!(self, Postgres::Local(_))
    }

    /// Stop what is held: the local cluster, cleanly; nothing for a
    /// remote.
    pub async fn stop(self) {
        if let Postgres::Local(local) = self {
            local.stop().await;
        }
    }
}

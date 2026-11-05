//! The host's identities: listed, each with its tags, from the files as
//! they are then; their tags put on and taken off; and one signed in on
//! this host.

use diverge_sdk::daemon::endpoints::identities::list::server::response::Identity;
use diverge_sdk::shared;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::store;

pub mod list;
pub mod login;
pub mod tag;
pub mod untag;

/// Every identity on the host, as a list answers it: each file's issuer
/// and subject, by issuer and then subject, with the tags the daemon
/// keeps on it. The files that could not be read are the inner error.
pub(crate) async fn held(conn: &mut PgConnection, daemon: &Daemon) -> Result<Result<Vec<Identity>, String>, store::Error> {
    let names = match daemon.authenticator.identities().list().await {
        Ok(names) => names,
        Err(error) => return Ok(Err(error.to_string())),
    };
    let mut tagged = store::identities::all(conn).await?;
    Ok(Ok(names
        .into_iter()
        .map(|name| {
            let key = shared::Identity { issuer: name.issuer, subject: name.subject };
            let tags = tagged.remove(&key).unwrap_or_default();
            Identity { issuer: key.issuer, subject: key.subject, tags }
        })
        .collect()))
}

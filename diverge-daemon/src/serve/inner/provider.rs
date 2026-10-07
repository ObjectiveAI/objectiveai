//! The providers a container names, checked to be on record.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use sqlx::PgConnection;

use super::Checked;
use crate::store::{self, providers_incoming, providers_outgoing};

/// Every provider the create names — the pin, and the provider of
/// each cross-provider mount — on record: `Error` naming the first
/// that is not, else nothing.
pub async fn providers(
    conn: &mut PgConnection,
    pinned: Option<&Provider>,
    file_mounts: &[FuseMount],
    directory_mounts: &[FuseMount],
) -> Result<Checked<()>, store::Error> {
    let named = pinned
        .map(|provider| &provider.identity)
        .into_iter()
        .chain(file_mounts.iter().map(|mount| &mount.provider))
        .chain(directory_mounts.iter().map(|mount| &mount.provider));
    for identity in named {
        if !on_record(conn, identity).await? {
            return Ok(Checked::Error(format!("no provider is known as {}", describe(identity))));
        }
    }
    Ok(Checked::Ok(()))
}

/// Whether the daemon knows a provider by the identity.
pub async fn on_record(conn: &mut PgConnection, identity: &Identity) -> Result<bool, store::Error> {
    Ok(match identity {
        Identity::Outgoing { address } => providers_outgoing::by_address(conn, address, false).await?.is_some(),
        Identity::IncomingUnbrokered { identity } => providers_incoming::by_identity(conn, identity, false).await?.is_some(),
    })
}

/// The identity in a sentence.
fn describe(identity: &Identity) -> String {
    match identity {
        Identity::Outgoing { address } => format!("the outgoing provider at {address}"),
        Identity::IncomingUnbrokered { identity } => format!("the incoming provider {identity}"),
    }
}

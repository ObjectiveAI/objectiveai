//! A provider that dialled in, held for its connection's life.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::connection::Connection;

use super::{Error, attach};
use crate::daemon::Daemon;

/// Serve `connection`, a provider admitted at the handshake as
/// `identity` by the credential hashing to `key_hash`, until it ends:
/// attached under `Identity::IncomingUnbrokered`, holding the
/// credential, and served. Why it ended, or why it never attached —
/// another connection holding the identity or the credential, or a
/// socket that is not a provider's — is nobody's to hear: the
/// provider dialled, and gets its socket closed and nothing else.
pub async fn incoming(connection: Connection, identity: String, key_hash: String, daemon: &Arc<Daemon>) -> Result<(), Error> {
    let attached = attach(connection, Identity::IncomingUnbrokered { identity }, Some(key_hash), daemon).await?;
    attached.serve().await;
    Ok(())
}

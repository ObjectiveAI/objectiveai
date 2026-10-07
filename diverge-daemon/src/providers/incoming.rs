//! A provider that dialled in, held for its connection's life.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::connection::Connection;

use super::{Error, attach};
use crate::daemon::Daemon;

/// Serve `connection`, a provider admitted at the handshake as
/// `identity`, until it ends: attached under
/// `Identity::IncomingUnbrokered`, and served. Why it ended, or why
/// it never attached, is nobody's to hear: the provider dialled, and
/// a provider that is not one gets its socket closed and nothing
/// else.
pub async fn incoming(connection: Connection, identity: String, daemon: &Arc<Daemon>) -> Result<(), Error> {
    let attached = attach(connection, Identity::IncomingUnbrokered { identity }, daemon).await?;
    attached.serve().await;
    Ok(())
}

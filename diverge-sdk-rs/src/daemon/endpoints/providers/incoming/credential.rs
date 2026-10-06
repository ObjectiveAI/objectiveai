//! How a provider dials in: the identity it has, and from where.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A credential a client dials in with: the identity it names, and the
/// one address it is accepted from, if any. The KEY is not here, on any
/// request or in any report: the daemon mints it when the credential is
/// made — on an
/// [`add`](crate::daemon::endpoints::providers::incoming::add), or an
/// [`edit`](crate::daemon::endpoints::providers::incoming::edit) that
/// replaces it — answers it in that response, once, and never reports
/// it again; a client presents it byte for byte, and the daemon knows
/// it by the key alone. One credential per identity. One JSON object,
/// `{"identity":…}` with an `address` if any.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Credential {
    /// Who a provider presenting the key is: its identity from then on,
    /// as
    /// [`Identity::IncomingUnbrokered`](crate::daemon::endpoints::agents::logs::server::response::Identity::IncomingUnbrokered)
    /// carries it. Compared and not read.
    pub identity: String,
    /// The one peer address the key is accepted from, as the OS reports
    /// it; absent, any address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<IpAddr>,
}

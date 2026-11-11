//! One route to a listed identity: a provider that lists it, and
//! where.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// One provider whose `daemons::list` holds the identity, with where
/// that provider holds its connection: what a
/// [`daemons`](crate::daemon::endpoints::daemons) record's filters
/// choose among.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Via {
    /// The provider, by its identity.
    pub provider: Identity,
    /// The other end of the identity's connection to the provider: the
    /// IP it dialled the provider from, or the IP the provider reached
    /// it at.
    pub ip: IpAddr,
    /// The dial target the provider dialled the identity at, as the
    /// provider's configuration writes it — a `host:port` or a URL —
    /// when the provider dialled it; absent when it dialled the
    /// provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Whether the identity listens for messages through the provider:
    /// a [`providers::daemons::message`](crate::daemon::endpoints::providers::daemons::message) to it
    /// goes only through a provider where it listens.
    pub listening: bool,
}

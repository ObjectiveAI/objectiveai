//! The document itself.

use std::collections::BTreeMap;

use diverge_sdk::container_proxy::outside::OUTSIDE_PORT;
use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

use super::clients::Client;
use super::containers::Containers;
use super::images::Images;
use super::volumes::Volumes;

/// The whole of `provider.yaml`: what the provider is told.
///
/// An absent file is this type's [`Default`], and the provider runs
/// on it: the proxy's own port, no identity of its own, no peer
/// accepted, no peer dialled, no volume, no client image store, and the
/// [`Containers`] defaults. Every section may be absent, meaning what
/// its doc says. An unknown key is an error, so a misspelled
/// setting is refused rather than ignored.
///
/// Every path inside the file resolves relative to the root, the
/// directory that holds `provider.yaml`, and never to the working
/// directory: [`load`](crate::config::load) resolves each as it
/// reads, so what a program holds is absolute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// The TCP port the provider accepts the protocol on: a WebSocket
    /// upgrade at any path, on every interface. Absent means the port
    /// the container proxy listens on inside a container, `14979`,
    /// which nothing on the host takes: a container's proxy is
    /// published to a loopback port podman picks.
    #[serde(default = "proxy_port")]
    pub port: u16,
    /// The provider's own identities: for each issuer, the subject
    /// whose token it presents there, in every handshake from that
    /// issuer — those it answers and those it makes. An issuer absent
    /// here is one the provider trusts no token from, since it could not
    /// answer from it. Each is an identity of the root's
    /// [`identities`](diverge_sdk::identities), signed in beforehand.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub identities: BTreeMap<String, String>,
    /// The identities the provider serves: a peer whose verified
    /// `(issuer, subject)` is not here is closed in the handshake —
    /// whichever end dialled, with no distinction between the two.
    /// Absent, nobody is served.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<Identity>,
    /// The peers the provider dials: see [`Client`]. Absent, it dials
    /// no one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clients: Vec<Client>,
    /// The runtime the provider runs containers with, and what it
    /// gives it. Absent from the file means its [`Default`]: a
    /// provider runs containers before it has configured anything.
    #[serde(default)]
    pub containers: Containers,
    /// Where volumes may be created, and which exist already. Absent
    /// means no volume can be created and none exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Volumes>,
    /// Where the images clients build on the provider are kept, and
    /// how much they may take. Absent means no client image store: a
    /// build is refused, a listing is empty, and the `client` source
    /// of a check is never available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Images>,
}

/// What a provider runs on before it has written a line of
/// configuration: the proxy's port, no identity, nobody accepted,
/// nobody dialled,
/// the [`Containers`] defaults, no volume, no client image store.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: proxy_port(),
            identities: BTreeMap::new(),
            accepted: Vec::new(),
            clients: Vec::new(),
            containers: Containers::default(),
            volumes: None,
            images: None,
        }
    }
}

/// The port `port` takes when the file leaves it out.
fn proxy_port() -> u16 {
    OUTSIDE_PORT
}

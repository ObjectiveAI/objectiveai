//! The document itself.


use diverge_sdk::container_proxy::outside::OUTSIDE_PORT;
use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

use super::outgoing::Outgoing;
use super::containers::Containers;
use super::images::Images;
use super::messages::Messages;
use super::volumes::Volumes;

/// The whole of `provider.yaml`: what the provider is told.
///
/// An absent file is this type's [`Default`], and the provider runs
/// on it: the proxy's own port, no identity of its own, no peer
/// admitted, no peer dialled, no volume, no Store, and the
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
    /// Who the provider is: the one identity whose token it presents
    /// in every handshake — those it answers and those it makes —
    /// whatever issuer the peer's is from. It is an identity of the
    /// root's [`identities`](diverge_sdk::identities), signed in
    /// beforehand. It lists no issuer the provider trusts: whom the
    /// provider serves is `incoming`'s and `outgoing`'s to say, from
    /// whatever issuer each is. Absent, the provider has no identity, and completes no
    /// handshake.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    /// The identities that may dial the provider: a peer that dials
    /// in and whose verified `(issuer, subject)` is not here is closed
    /// in the handshake. It says nothing of the peers the provider
    /// dials, each of which names its own. Absent, nobody dials in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incoming: Vec<Identity>,
    /// The peers the provider dials, each with the identity that must
    /// answer: see [`Outgoing`]. Absent, it dials no one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outgoing: Vec<Outgoing>,
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
    /// how much they may take. Absent means no Store: a
    /// build is refused, a listing is empty, and no image is held for
    /// a caller but one a registry the provider uses serves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Images>,
    /// The rate the provider relays direct messages at, between the
    /// identities connected to it. Absent means it relays none: a
    /// `daemons::listen` and a `daemons::message` are each answered
    /// unsupported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<Messages>,
}

/// What a provider runs on before it has written a line of
/// configuration: the proxy's port, no identity, nobody admitted,
/// nobody dialled,
/// the [`Containers`] defaults, no volume, no Store, no
/// messages.
impl Default for Config {
    fn default() -> Self {
        Config {
            port: proxy_port(),
            identity: None,
            incoming: Vec::new(),
            outgoing: Vec::new(),
            containers: Containers::default(),
            volumes: None,
            images: None,
            messages: None,
        }
    }
}

/// The port `port` takes when the file leaves it out.
fn proxy_port() -> u16 {
    OUTSIDE_PORT
}

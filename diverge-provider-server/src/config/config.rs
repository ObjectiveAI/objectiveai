//! The document that is `config.yaml`.

use serde::{Deserialize, Serialize};

use super::auth::Auth;
use super::clients::Clients;
use super::containers::Containers;
use super::volumes::Volumes;

/// The whole of `config.yaml`.
///
/// An absent file is this type's [`Default`], and the provider runs
/// on it: no peer accepted, no peer dialled, no volume, and the
/// [`Containers`] defaults. `containers` is always present — absent
/// from the file, it is its [`Default`] — and the other three sections
/// are optional, an absent one meaning what its doc says. An unknown
/// key is an error, so a misspelled setting is refused rather than
/// ignored.
///
/// Every path inside the file resolves relative to the provider's
/// directory, the one that holds the file, and never to the working
/// directory. The resolution happens when the file is READ; this type
/// holds each path as it was written.
///
/// The document is also what the provider writes back, so nothing in
/// it depends on comments surviving a round trip.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// How a peer that dials the provider is judged. Absent means no
    /// dialling peer is accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    /// The peers the provider dials. Absent means the provider dials
    /// no one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clients: Option<Clients>,
    /// The runtime the provider runs containers with, and what it
    /// gives it. Absent from the file means its [`Default`]: a
    /// provider runs containers before it has configured anything.
    #[serde(default)]
    pub containers: Containers,
    /// Where volumes may be created, and which exist already. Absent
    /// means no volume can be created and none exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Volumes>,
}

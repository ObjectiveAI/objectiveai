//! The document that is `config.yaml`.

use serde::{Deserialize, Serialize};

use super::auth::Auth;
use super::clients::Clients;
use super::volumes::Volumes;

/// The whole of `config.yaml`.
///
/// Every section is optional, and an absent section is its defaults;
/// an absent file is this type's [`Default`]. An unknown key is an
/// error, so a misspelled setting is refused rather than ignored.
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
    /// Where volumes may be created, and which exist already. Absent
    /// means no volume can be created and none exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Volumes>,
}

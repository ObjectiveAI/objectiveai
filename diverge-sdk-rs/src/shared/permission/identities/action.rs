//! What may be done to the host's identities.

use serde::{Deserialize, Serialize};

/// An act over the host's identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// List the issuers an identity is signed in at, as
    /// [`identities::issuers`](crate::daemon::endpoints::identities::issuers)
    /// does.
    Issuers,
    /// List the subjects signed in at one issuer, as
    /// [`identities::subjects`](crate::daemon::endpoints::identities::subjects)
    /// does.
    Subjects,
    /// Sign one in on the host, as
    /// [`identities::login`](crate::daemon::endpoints::identities::login)
    /// does: the one act that writes.
    Login,
}

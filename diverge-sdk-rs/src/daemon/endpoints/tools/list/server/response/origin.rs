//! Where a tool comes from: made by this caller, or somebody else's
//! joined.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::{Identity, Provider};

/// The two ways a tool comes to be, and what the daemon knows of
/// each: a [`create`](crate::daemon::endpoints::tools::create) made
/// it from an image, or a
/// [`connect`](crate::daemon::endpoints::tools::connect) named a
/// container somebody else runs. JSON-tagged by `kind`, `created` or
/// `connected`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// Made by this caller's create: the daemon runs its container.
    Created {
        /// The template it was made from, by id: the hash a
        /// [`templates::create`](crate::daemon::endpoints::tools::templates::create)
        /// answered.
        template: String,
        /// The provider the container runs on, if it runs, or last
        /// ran on; absent for a tool that has never run. See
        /// [`Provider`].
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider: Option<Provider>,
        /// The container's id while it runs, as the provider's run
        /// answered it: what this caller hands, with the provider and
        /// an authorization, to whoever it lets connect. Absent while
        /// the container does not run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Somebody else's container, joined by this caller's connect:
    /// the daemon holds a connect scope on it while an attached agent
    /// is active, and never starts or stops it.
    Connected {
        /// The provider the container runs on, as the connect named
        /// it.
        provider: Identity,
        /// The container's id, as the connect named it.
        id: String,
    },
}

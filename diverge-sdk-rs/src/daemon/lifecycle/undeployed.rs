//! A run's container down.

use serde::{Deserialize, Serialize};

use super::{Provider, Reason};

/// The run's container went down: the last item of every run — see
/// rule 3 of [`lifecycle`](super). The provider is the one its
/// [`Deployed`](super::Deployed) named, and its members lie beside the
/// item's own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Undeployed {
    /// The provider the run was deployed on.
    #[serde(flatten)]
    pub provider: Provider,
    /// Why the run ended unexpectedly; absent when it ended because it
    /// was asked to — see rule 4 of [`lifecycle`](super).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
    /// Whether the run was working when it ended: an agent's loop or
    /// an exchange in flight, a tool's ask open. When true, an
    /// `inactive` precedes this item.
    pub interrupted: bool,
}

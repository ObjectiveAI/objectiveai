//! A run's container up on a provider.

use serde::{Deserialize, Serialize};

use super::Provider;

/// The run's container came up on the provider named: the first item
/// of every run, before anything the run did — see rule 1 of
/// [`lifecycle`](super). The provider's members lie beside the item's
/// own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Deployed {
    /// The provider the run is deployed on.
    #[serde(flatten)]
    pub provider: Provider,
}

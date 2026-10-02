//! The daemon's tool that edits agents.

use serde::{Deserialize, Serialize};

/// That an agent has the tool that edits agents. An empty object, for now:
/// what the tool does, and how far it reaches, is not yet stated
/// here, and a member will land here when it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentsEdit {}

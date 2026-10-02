//! The daemon's tool over agents as they are.

use serde::{Deserialize, Serialize};

/// That an agent, or a tool, has the tool over agents as they are. An empty object, for now:
/// what the tool does, and how far it reaches, is not yet stated
/// here, and a member will land here when it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Agents {}

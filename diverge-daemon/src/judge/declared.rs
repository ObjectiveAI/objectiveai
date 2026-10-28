//! A tool as it is served: the grants its container declared, and
//! which tool it is, whose attached agents `attached` reaches.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::grant::{self, Converted, Grant};

use crate::store::{DependencyToolId, ToolId};

/// What a tool's container declared at its registration, fixed for
/// the run: the creator its requests make things as, the daemon's
/// grants its declaration converts to, and the actions over agents it
/// holds within `attached`, which no daemon grant can say and which
/// are judged against the agents attached to the tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixed {
    /// What the tool's requests make things as: the tool itself, a
    /// `Creator::Tool`.
    pub creator: Creator,
    /// Every grant a role could hold, in no order.
    pub grants: Vec<Grant>,
    /// The actions over agents held within `attached`, in no order.
    pub attached_actions: Vec<grant::agents::Over>,
}

impl Fixed {
    /// From a container's declared permissions, converted.
    pub fn new(creator: Creator, converted: Converted) -> Fixed {
        Fixed {
            creator,
            grants: converted.grants,
            attached_actions: converted.attached_actions,
        }
    }
}

/// Which tool a run is, by its row: a standalone tool or a dependency
/// tool, whose attached agents are read differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRef {
    /// A standalone tool: attached to agents directly, and to every
    /// agent of the agent templates it is attached to.
    Standalone(ToolId),
    /// A dependency tool: attached to the agents whose registration
    /// declared its template, and still does.
    Dependency(DependencyToolId),
}

/// A tool, as its `/daemon` connections are served: what its container
/// declared, and which tool it is, whose attached agents — read fresh
/// for every request — are what `attached` reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// What the container declared.
    pub fixed: Arc<Fixed>,
    /// The tool, by its row: whose attachments `attached` reaches.
    pub tool: ToolRef,
}

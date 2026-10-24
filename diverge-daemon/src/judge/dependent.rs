//! A dependency tool as it is served: its template's grants, fixed,
//! and whose `self` it is.

use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::grant::{self, Converted, Grant};

use crate::store::DependencyToolId;

/// What a dependency tool template fixed for the tool's life: the
/// creator its requests make things as, the daemon's grants its
/// template converts to, and the actions over agents it holds within
/// `self`, which no daemon grant can say and which are judged against
/// the tool's own agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixed {
    /// What the tool's requests make things as: the tool itself, a
    /// `Creator::Tool` naming its template and whose it is.
    pub creator: Creator,
    /// Every grant a role could hold, in no order.
    pub grants: Vec<Grant>,
    /// The actions over agents held within `self`, in no order.
    pub self_actions: Vec<grant::agents::Over>,
}

impl Fixed {
    /// From a template's permissions, converted.
    pub fn new(creator: Creator, converted: Converted) -> Fixed {
        Fixed {
            creator,
            grants: converted.grants,
            self_actions: converted.self_actions,
        }
    }
}

/// A dependency tool, as its `/daemon` connections are served: what
/// its template fixed, and which tool it is, whose attached agents —
/// the one agent a transient tool was deployed for, every agent
/// attached to a service — are its `self`, read fresh for every
/// request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependent {
    /// What the template fixed.
    pub fixed: Arc<Fixed>,
    /// The tool, by its row: whose attachments `self` reaches.
    pub tool: DependencyToolId,
}

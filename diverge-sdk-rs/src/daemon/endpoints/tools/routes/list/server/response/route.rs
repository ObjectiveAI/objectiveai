//! One route, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::tools::routes::Path;

/// One route of the caller's: the position, the tool served there, when
/// it was put down, and by whom.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Route {
    /// The position: see [`Path`].
    pub path: Path,
    /// The tool served there, as it was when routed: its template, its
    /// index and its name. The template is the path's.
    pub tool: creator::Tool,
    /// When the set put it down.
    pub created: DateTime<Utc>,
    /// Who put it down: the client, over an endpoint, or the agent or
    /// the tool of the client's that did so through the daemon. One
    /// [`Creator`](crate::daemon::creator::Creator), the direct maker;
    /// the maker's own maker is on the maker's list item.
    pub creator: Creator,
}

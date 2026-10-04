//! What narrows a list of routes, and what a permission over them
//! reaches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;

/// The filter over routes: every member optional, and every one given a
/// condition a route passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends, and its program transforms what passes. In
/// the daemon's own tools, as a
/// [permission](crate::daemon::daemon_tools), the same filter says
/// which routes the tool reaches, and its program is a test: a route
/// passes when the program, run with the route as its input, yields
/// first a value that is neither `false` nor `null`. A filter with no
/// member given passes every route.
///
/// # Any one of
///
/// A member that lists candidates matches a route that is any one of
/// them, or was made by any one of them. An empty list is absent,
/// and matches every route.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Paths beginning at any one of these agents, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Paths ending at any one of these templates, by id: the
    /// dependency's template.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Routed to any one of these tools, by name as the tool is called
    /// now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Put down by any one of these, directly: see
    /// [`Creator`](crate::daemon::creator::Creator). Absent when empty,
    /// and then made by anybody.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creators: Vec<Creator>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching route — one
    /// [`Route`](crate::daemon::endpoints::tools::routes::list::server::response::Route)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. So `.path.agent` is every matching route's agent, as a
    /// string. Absent, the routes come back as they are. The daemon
    /// does not read the program beyond running it; one that will not
    /// compile, or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
}

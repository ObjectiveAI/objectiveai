//! The daemon's own tools, as a template hands them to the agents
//! and the tools made from it.
//!
//! Besides the tool containers attached to an agent, an agent — or a
//! tool container the daemon runs — may call tools the daemon itself
//! answers: the daemon's own verbs over the caller's agents, tools,
//! and the templates both are made from. Which of them a container
//! has, and how far each reaches, is its template's: the
//! [`DaemonTools`] a [template](crate::daemon::template) carries
//! names each tool the containers made from it hold, with its
//! reach, and a container holds exactly the tools named and no
//! other. A template that carries no `daemon_tools` makes
//! containers that hold none.
//!
//! # The filter as a permission
//!
//! Each tool's reach is a filter — the very shape the list of that
//! family narrows by,
//! [`agents::list`](crate::daemon::endpoints::agents::list::client::request::Filter),
//! [`tools::list`](crate::daemon::endpoints::tools::list::client::request::Filter),
//! and the two template lists' — read as a test rather than a
//! narrowing: a thing is within the tool's reach when it passes every
//! member of the filter that is given, and, when the filter carries a
//! program, when the program run with the thing as its input yields
//! first a value that is neither `false` nor `null`. A program that
//! yields nothing, or fails, passes nothing. The program transforms
//! nothing here: what the tool answers is the thing as it is. A filter
//! with no member given passes every thing of the caller's.
//!
//! A tool that acts on one thing — a delete, an edit, a message, a
//! tag — is refused on a thing its filter does not pass, and the
//! thing is as it was. A tool that lists answers only what its
//! filter passes, narrowed further by the request's own filter. A
//! tool that makes something reaches templates: it makes from a
//! template its filter passes, and from no other. Each of these is
//! one filter, and is that filter on [`DaemonTools`]; the two tools
//! that join a tool to an agent carry a filter for each, and are
//! [`ToolsAttach`] and [`ToolsDetach`]; the four that tag and untag
//! carry a filter and the [`Tags`] they may act with, any or only
//! those named, and are [`AgentsTag`], [`AgentsUntag`], [`ToolsTag`]
//! and [`ToolsUntag`].
//!
//! What each tool says to its caller, and how the daemon knows who
//! is calling, is not yet stated here.

mod agents_tag;
mod agents_untag;
mod daemon_tools;
mod tags;
mod tools_attach;
mod tools_detach;
mod tools_tag;
mod tools_untag;

pub use agents_tag::*;
pub use agents_untag::*;
pub use daemon_tools::*;
pub use tags::*;
pub use tools_attach::*;
pub use tools_detach::*;
pub use tools_tag::*;
pub use tools_untag::*;

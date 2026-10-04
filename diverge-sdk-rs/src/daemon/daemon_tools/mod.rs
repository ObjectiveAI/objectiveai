//! The daemon's own tools, as a create hands them to the agent or
//! the tool it makes.
//!
//! Besides the tool containers attached to an agent, an agent — or a
//! tool container the daemon runs — may call tools the daemon itself
//! answers: the daemon's own verbs over the caller's agents, tools,
//! and the templates both are made from, and over its resources.
//! Which of them a container has, and how far each reaches, is its
//! create's and not its template's: an
//! [agent's create](crate::daemon::endpoints::agents::create) and a
//! [tool's create](crate::daemon::endpoints::tools::create) carry, in
//! their shared [`Inner`](crate::daemon::create::Inner), one member
//! per tool, with its reach — `disabled`, `any`, or `only` what it
//! names, a [`Reach`] — and a container holds exactly the tools not
//! `disabled`. Which
//! tools it holds is fixed for its life; how far each reaches is
//! edited. A create that carries no `daemon_tools` makes a container
//! that holds none, and a connected tool holds none: it is somebody
//! else's container, and calls nothing of the caller's.
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
//! template its filter passes, and from no other. `any` is every
//! thing of the caller's with no filter at all. Each of these is
//! one filter, and is that filter on the create; the two tools
//! that join a tool to an agent carry a filter for each, and are
//! [`ToolsAttach`] and [`ToolsDetach`]; the eight that tag and untag
//! are [`Held`] rather than reached — `disabled`, or `only` with two
//! sides, which things and which tags, each a [`Within`], `any` or
//! `only` — and are [`AgentsTag`], [`AgentsUntag`],
//! [`AgentsTemplatesTag`], [`AgentsTemplatesUntag`], [`ToolsTag`],
//! [`ToolsUntag`], [`ToolsTemplatesTag`] and [`ToolsTemplatesUntag`].
//! The tools that make a template make something with no subject,
//! and are each a [`Switch`], `disabled` or `any`. Resources have no
//! list filter, so the tools that list and delete them reach `any`
//! resource or `only` those named by id. The tool that transfers
//! files reaches along edges — each a [`Source`] and a
//! [`Destination`], among the container's own filesystem, the tool
//! containers attached to it and the resources, see [`Edge`] — and a
//! transfer into the resources is an upload, the one way a container
//! has of making a resource. The three tools over
//! [routes](crate::daemon::endpoints::tools::routes) reach by a
//! filter too: the add by the tools list's, over the tools a
//! dependency may be routed to, and the delete and the list by the
//! routes list's own.
//!
//! What each tool says to its caller, and how the daemon knows who
//! is calling, is not yet stated here.

mod agents_tag;
mod agents_templates_tag;
mod agents_templates_untag;
mod agents_untag;
mod destination;
mod edge;
mod held;
mod reach;
mod source;
mod switch;
mod tools_attach;
mod tools_detach;
mod tools_tag;
mod tools_templates_tag;
mod tools_templates_untag;
mod tools_untag;
mod within;
mod word;

pub use agents_tag::*;
pub use agents_templates_tag::*;
pub use agents_templates_untag::*;
pub use agents_untag::*;
pub use destination::*;
pub use edge::*;
pub use held::*;
pub use reach::*;
pub use source::*;
pub use switch::*;
pub use tools_attach::*;
pub use tools_detach::*;
pub use tools_tag::*;
pub use tools_templates_tag::*;
pub use tools_templates_untag::*;
pub use tools_untag::*;
pub use within::*;

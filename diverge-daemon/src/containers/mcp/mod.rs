//! The one tool list a container sees, and the routing of its calls.
//!
//! A container's program sees its proxy as one MCP server. What that
//! server holds is the union of the tools the daemon serves the
//! container — the tools attached to it and the dependencies it
//! declared — each under a prefix of its own from the run's
//! [`Registry`], every MCP tool [`exposed`] as `<prefix>_<name>` and
//! every call routed back by its first `_`, [`split`]. [`Served`] is
//! the set for one run, changing live as tools are attached and
//! detached; [`list_tools`], [`list_resources`], [`call_tool`],
//! [`read_resource`] and [`notifications`] answer the five exchanges
//! over it, through a [`Context`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod merge;
mod prefix;
mod served;

pub use merge::*;
pub use prefix::*;
pub use served::*;

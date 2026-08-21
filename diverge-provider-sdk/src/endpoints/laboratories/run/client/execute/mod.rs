//! Performing the exchange, rather than describing it.
//!
//! [`execute`] runs a container and hands back two things:
//! [`ExecuteStream`], what the container reports about itself, and
//! [`ExecuteHandle`], everything a runner can say to it — including
//! [`stop`](ExecuteHandle::stop). [`RunFrame`] is what the stream
//! yields; [`ReadStream`] and [`McpStream`] are what two of the four
//! asks answer with.
//!
//! It is the only executor that takes two of what a caller supplies: an
//! [`OciProxy`](crate::client::oci_proxy::OciProxy) for an image, and a
//! [`LaboratoryConnectionAuthorizer`](crate::client::laboratory_connection_authorizer::LaboratoryConnectionAuthorizer)
//! for the connectors that turn up.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod execute;
mod execute_handle;
mod execute_stream;
mod mcp_frame;
mod mcp_stream;
mod read_stream;
mod run_frame;

pub use execute::*;
pub use execute_handle::*;
pub use execute_stream::*;
pub use mcp_frame::*;
pub use mcp_stream::*;
pub use read_stream::*;
pub use run_frame::*;

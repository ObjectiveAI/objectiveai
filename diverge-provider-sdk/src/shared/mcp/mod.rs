//! The five things an agent asks of an MCP server.
//!
//! [`list_tools`], [`list_resources`], [`call_tool`] and
//! [`read_resource`] are one exchange each: a request opens a channel
//! and one frame answers it. [`notifications`] is the fifth and the
//! odd one — nothing is asked, and frames arrive for as long as the
//! channel lives.
//!
//! # Why five, and why these five
//!
//! Because that is what an MCP server is, once the transport is taken
//! off it. Streamable HTTP has ONE url: a client POSTs a JSON-RPC
//! message to it for the first four, and opens a stream with a bare
//! `GET` on the same url for the fifth. The verb is the whole of the
//! distinction there; a tag byte is the whole of it here.
//!
//! # Why they are shared
//!
//! Because more than one endpoint carries them, in more than one
//! direction. An [`agentic_loop`](crate::endpoints::agentic_loop::run)
//! has an agent inside a container asking a caller's servers; an
//! [`mcp_plugin`](crate::endpoints::mcp_plugin::run) and a
//! [`laboratory`](crate::endpoints::laboratories) have a caller asking
//! a server the provider is holding.
//!
//! Same five exchanges, opposite ways round. A shape defined once per
//! endpoint would be four definitions that agree until they do not.
//!
//! # What is not here
//!
//! Initialization, capabilities, prompts, completion, sampling,
//! elicitation, and everything else MCP has. Not because they are
//! unimportant but because nothing carries them yet, and a frame
//! nothing sends is a frame nobody has had to be right about.
//!
//! Adding one is adding a module here and a variant to whichever
//! channel request wants it.

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;

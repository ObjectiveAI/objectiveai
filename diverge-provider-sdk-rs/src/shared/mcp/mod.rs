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
//! Because more than one scope carries them, in more than one
//! direction. Every [`containers`](crate::endpoints::containers) scope
//! has the container inside asking a caller's servers; a
//! [`tools`](crate::endpoints::containers::tools) scope also has the
//! caller asking a server the provider is holding; and the
//! [`proxy`](crate::container_proxy_endpoints::agents::begin) inside the container relays
//! the first of those one hop earlier.
//!
//! Same five exchanges, opposite ways round. A shape defined once per
//! endpoint would be four definitions that agree until they do not.
//!
//! # The image under `_meta`
//!
//! A container never addresses one server over another: it sees its
//! proxy, and the caller merges its servers into one list. So the
//! proxy says which image is which, under one `_meta` key,
//! `diverge.network/image`, whose value is an object of the
//! container's `name` and `digest` as the run request named them —
//! the proxy learns them on its begin. On the four requests a
//! program sends outward the proxy sets the key on the params, so a
//! tool container that receives the call knows which image is
//! calling. On what a tool container's own server answers — the
//! result of each of the four, each tool and each resource of a
//! list, and each notification — the proxy sets the key too, so a
//! program that reads a merged list knows which image serves each
//! tool. The proxy replaces a value the program set under that key
//! and leaves every other key as it was sent; the provider's server
//! relays all of it verbatim. The key's form is MCP's own rule for
//! `_meta` names: a prefix of dotted labels, a slash, a name.
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

mod frame_error;

pub use frame_error::*;

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;

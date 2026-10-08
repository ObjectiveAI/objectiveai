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
//! direction. Every [`containers`](crate::provider::endpoints::containers) scope
//! has the container inside asking a caller's servers; a
//! [`tools`](crate::provider::endpoints::containers::tools) scope also has the
//! caller asking a server the provider is holding; and the
//! [`proxy`](crate::container_proxy::outside::endpoints::agents::begin) inside the container relays
//! the first of those one hop earlier.
//!
//! Same five exchanges, opposite ways round. A shape defined once per
//! endpoint would be four definitions that agree until they do not.
//!
//! # Who is on the other side, under `_meta`
//!
//! A container never addresses one server over another: it sees its
//! proxy, and the caller merges its servers into one list. So the
//! caller says who is which, under three `_meta` keys — [`IMAGE`],
//! [`AGENT`], [`TOOL`], set by [`attest`] — and the caller is the
//! daemon. On the four requests a program sends outward the daemon
//! sets them on the params before it forwards the call, so a tool
//! container that receives it knows which agent is calling: its
//! image, its template and its index. On what a tool container's own
//! server answers — the result of each of the four, each tool and
//! each resource of a list, and each notification — the daemon sets
//! them as it relays the answer back, so a program that reads a
//! merged list knows which tool serves each entry. And on every chunk
//! an agent says, which is not MCP but has the same `_meta` at its
//! top level, the daemon sets them as it keeps the chunk, so a reader
//! of the log knows which agent spoke. The daemon replaces a value
//! set under those keys and leaves every other key as it was sent;
//! the proxy and the provider's server set nothing under `_meta` and
//! relay all of it as sent. The keys' form is MCP's own rule for
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
mod meta;

pub use frame_error::*;
pub use meta::*;

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;

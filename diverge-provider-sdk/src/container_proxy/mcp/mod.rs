//! The `/mcp` path: the container's tool calls, answered by the
//! caller's MCP servers.
//!
//! To the agent inside an agent container, the proxy is a fully
//! compliant MCP server at `/mcp/agent` on the same port. Every
//! exchange the agent asks of it leaves the container on this path,
//! and the caller's servers answer on the far side of the provider.
//!
//! An exchange path (see [the module](super)): the container opens a
//! channel with one [`Request`](request::Request), the server answers
//! with the exchange's responses and the finish.
//!
//! ```text
//! container → server:  [channel: u8][tag: u8][params JSON…]
//! server → container:  [type: u8][channel: u8][payload…]
//! ```
//!
//! # The request is typed, and the responses are not
//!
//! A [`Request`](request::Request) is one of the five MCP exchanges,
//! as [`shared::mcp`](crate::shared::mcp) defines them, with a tag
//! byte saying which. A response stays bytes, and not for want of a
//! type: which exchange a response answers is known only to whoever
//! opened the channel, and the payload's own tag — result or error —
//! discriminates within an exchange, not between them. The opener
//! decodes with the response type of the exchange it asked for.
//!
//! Four of the five are answered once and finished; notifications is
//! a stream — one response per notification for as long as the
//! channel lives — because in MCP it is not a method but the place a
//! server pushes into.
//!
//! # A connection dying does not fail an exchange
//!
//! An exchange is answered when its response has arrived AND its
//! channel has finished. A channel that died before that point left
//! the exchange un-answered, and the container asks it again on the
//! next connection, as a fresh channel. So a caller may legitimately
//! receive the same logical ask twice, and an ask with side effects
//! may be performed twice; that is the chosen trade, because the
//! agent inside is waiting and the alternative is telling it a
//! transport story it can do nothing about. The one non-answer that
//! is not re-asked is the deliberate one: a finish with no response
//! is the far side's statement, not an accident.
//!
//! # Types
//!
//! | type | server |
//! |------|--------|
//! | 0    | channel response |
//! | 1    | channel response finish |

pub mod request;
pub mod response;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;

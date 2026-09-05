//! The `/mcp/notifications` path: what the caller's MCP servers say
//! on their own account, pushed to the container.
//!
//! The one MCP path that is not an exchange. Nothing is asked: the
//! server connects and sends one [`response::Frame`] per
//! notification — tools changed, resources changed, a resource
//! updated, a log line — for as long as the connection lives, and
//! the container sends nothing. A stream path (see [the
//! module](super::super)), in the direction opposite to
//! [`filetree`](super::super::filetree)'s.
//!
//! ```text
//! server → container:  [notification JSON…]
//! ```
//!
//! No header: the server sends exactly one kind of frame, so there
//! is no type byte and no channel to spend, and the frame is the
//! [`shared::mcp::notifications`](crate::shared::mcp::notifications)
//! response as that module encodes it.
//!
//! # Why nothing is asked
//!
//! In Streamable HTTP a client opens the notification stream with a
//! bare `GET`: no method, no body, nothing to say. The container had
//! nothing to put in a request but the wish to listen, and the
//! connection existing is that wish. It also spares the container
//! the one exchange the old wire opened before the agent had asked
//! for anything — a server that has notifications sends them, and
//! one that has none sends nothing, at no cost to anyone.
//!
//! # Every connection is a fresh stream
//!
//! Notifications are not replayed: a connection carries what the
//! servers say from the moment it exists. An
//! [`Error`](crate::shared::mcp::notifications::response::Frame::Error)
//! is the caller's stream ending and is the last frame; the server
//! then closes, and the next connection starts over.

pub mod response;

mod error;

pub use error::FrameError;

//! Commands: the container asks the caller to run a diverge command,
//! and the answer streams back.
//!
//! A container has no CLI binary and no daemon it may dial, so a
//! command it wants run has to be run by somebody who can. That is
//! the caller. The container asks with a
//! [`Command`](crate::container_proxy::requests::Request::Command) on
//! `/requests` (kind `5`, the command's bytes), and the server
//! answers on `/command/{channel}`: one message per
//! [`Response`] the command produces, then the close.
//!
//! ```text
//! the ask, after the channel:   [5][command…]
//! the answer, per message:      [tag: u8][item… | error JSON…]
//! ```
//!
//! # Opaque, deliberately
//!
//! The command vocabulary is not this specification's to define. It
//! belongs to the CLI, which gains subcommands on its own schedule,
//! and a wire that named them would have to be revised every time
//! one appeared. So a command is bytes and an item is bytes, and
//! nothing between the container and the CLI reads either. What a
//! container may ask for is settled between it and the caller.
//!
//! # One ask, then a stream, then the close
//!
//! A command yielding a thousand rows delivers them as a thousand
//! messages, each as it lands, and the close is the end of the
//! command. An [`Error`](Response::Error) is the caller saying the
//! command did not finish; it is the last message when present. The
//! first message or the close is the acknowledgement, and nothing
//! times anything out.
//!
//! # No retry
//!
//! A command has effects. One whose answer never came, or whose path
//! died mid-stream, is a command whose outcome is unknown, and it is
//! reported to whoever asked as failed rather than run again.

mod response;

pub use response::*;

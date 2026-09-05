//! The `/command` path: the container asks the caller to run a
//! diverge command, and the answer streams back.
//!
//! A container has no CLI binary and no daemon it may dial, so a
//! command it wants run has to be run by somebody who can. That is
//! the caller. The container asks, the server relays, the caller
//! executes, and the answers come back as they are produced.
//!
//! An exchange path (see [the module](super)): the container opens a
//! channel with one command, the server answers with as many
//! [`Response`](response::Response)s as the command produces items,
//! then the finish.
//!
//! ```text
//! container → server:  [channel: u8][command…]
//! server → container:  [type: u8][channel: u8][tag: u8][item… | error…]
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
//! # One ask, then a stream
//!
//! A command yielding a thousand rows delivers them as a thousand
//! responses, each as it lands, and the channel's finish is the end
//! of the command — there is no terminator in the payload. An
//! [`Error`](response::Response::Error) is the caller saying the
//! command did not finish; it is the last response when present.
//! The first item or the finish is the acknowledgement, and nothing
//! times anything out.
//!
//! # No retry law
//!
//! A command has effects. A channel that died with its connection is
//! a command whose outcome is unknown, and it is reported to whoever
//! asked as failed rather than run again.
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

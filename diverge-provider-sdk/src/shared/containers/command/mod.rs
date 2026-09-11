//! A command the container wants run, and the items it produced.
//!
//! A container has no CLI binary and no daemon it may dial, so a
//! command it wants run has to be run by somebody who can. That is the
//! caller. The ask is [`request::Request`], the command's bytes; the
//! answer is a stream of [`response::Frame`]s — one per item the
//! command produced, then the finish, or an
//! [`Error`](response::Frame::Error) last when it did not finish.
//!
//! # Opaque, deliberately
//!
//! The command vocabulary is not this specification's to define. It
//! belongs to the CLI, which gains subcommands on its own schedule,
//! and a wire that named them would have to be revised every time one
//! appeared. So a command is bytes and an item is bytes, and nothing
//! between the container and the CLI reads either. What a container
//! may ask for is settled between it and the caller.
//!
//! # One frame per item
//!
//! Not one frame per command. A command that yields a thousand rows
//! sends a thousand of these and then finishes, so the container sees
//! each as it lands rather than waiting for a document assembled from
//! all of them. The finish is the end of the command, and there is no
//! terminator in the payload because a second signal for one fact is
//! a second thing to disagree about.
//!
//! The same shapes ride both wires this crate defines: the provider's
//! channel toward the caller, and the
//! [`proxy`](crate::container_proxy::command) inside the container.

pub mod request;
pub mod response;

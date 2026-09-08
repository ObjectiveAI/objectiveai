//! The `/run-loop` path: the loop, run.
//!
//! Opened by the server, for an agent container. It sends exactly one
//! message — the [`request::Request`], the prompt and the agent the
//! container was made with — and the container answers with the loop
//! as it happens: one [`response::Frame`] per chunk, then the clean
//! close, which is the loop ended. An [`Error`](response::Frame::Error)
//! is a message too — first, when nothing ran, or last, when the loop
//! died with nothing more to say — and the close follows it.
//!
//! ```text
//! server → container:   [request JSON]                       once
//! container → server:   [0][chunk JSON] …                     then the close
//!                    or [0][chunk JSON] … [1][error JSON]     then the close
//! ```
//!
//! # The proxy is a relay
//!
//! The loop is a program beside the proxy — the harness — and the
//! proxy carries the request to it and its frames back untouched. The
//! harness attaches at `/run-loop/agent`, receives the request as its
//! first message, and everything it sends after is a frame of this
//! path, verbatim; the proxy decodes none of it. One frame the proxy
//! sends on its own: a harness vanishing mid-stream — its socket
//! ending without a close — is an `Error` saying so, then the close,
//! because a stream that simply stopped could not be told from one
//! that finished.
//!
//! # One at a time, and nobody waits in vain
//!
//! A container runs one loop; a second opening while one is in
//! progress is refused with `409` before the upgrade. A server that
//! opens before a harness has attached waits for one — nothing times
//! anything out — and a harness that attaches with no server yet
//! waits for the request the same way. Whichever side leaves first
//! ends the other's wait.
//!
//! # What is an error, and what is not
//!
//! Anything that fails before the loop has said a single thing — an
//! agent value the image will not take, a key it does not have, a
//! history it cannot open, the first fetch — is an `Error`, then the
//! close: there was no loop to report on. A failure after output is a
//! fatal notification chunk, part of the output, then the close —
//! the loop's own vocabulary, as
//! [`shared::containers::run_loop`](crate::shared::containers::run_loop)
//! states it.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;

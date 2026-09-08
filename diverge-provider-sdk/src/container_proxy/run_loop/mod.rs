//! The `/agent/run` path: the loop, run.
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
//!                    or [1][error JSON]                        then the close
//!                    or [0][chunk JSON] … [1][error JSON]     then the close
//! ```
//!
//! # The proxy forwards
//!
//! The loop is the agent's server's, at
//! [`agent::port()`](super::agent::port): the proxy `POST`s the
//! request to its `/run`, verbatim — it decodes none of it — and
//! answers what comes back. A `2xx` is a stream of chunks, `text/
//! event-stream`, and every event's data goes out as one `Chunk`,
//! its JSON untouched; the stream's end is the clean close. A
//! non-`2xx`, or a server that cannot be dialed, is the `Error`
//! frame first — the loop never ran — as [`agent`](super::agent)
//! states. One frame the proxy sends on its own: the stream dying
//! mid-way — the connection to the agent's server ending without the
//! stream's end — is an `Error` saying so, last, because a stream
//! that simply stopped could not be told from one that finished.
//!
//! One loop at a time is the agent's server's rule, not the proxy's:
//! a second `/run` while one is in progress is its own non-`2xx`,
//! forwarded like any other.
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
//! states it. The stream from the agent's server never carries an
//! error event; the two ways it can fail are the status before it
//! and the notification within it.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;

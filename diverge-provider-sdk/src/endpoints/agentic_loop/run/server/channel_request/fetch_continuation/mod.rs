//! Asking for the continuation the run resumes from.
//!
//! The fourth fetch, and the one with nothing to name: a run resumes
//! from exactly one thing — the continuation its caller holds, the
//! provider's own opaque state from an earlier run's close — so the
//! channel says it whole and the [`Request`] carries nothing. A
//! provider opens it when the run starts, and the client answers
//! with the bytes themselves:
//! [`fetch_continuation::Frame`](crate::endpoints::agentic_loop::run::client::channel_response::fetch_continuation::Frame)s
//! of at most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE)
//! each — the same chunks the earlier run closed with, in the same
//! order, one frame each — then the finish.
//!
//! # The empty finish is a fresh start
//!
//! Unlike the other fetches, zero frames before the finish is not a
//! failure to serve — it is the answer "there is nothing to resume":
//! the caller starts the conversation here. A client with no
//! continuation ends the response at once, and that is the common
//! case, not the exceptional one. There is no error vocabulary on
//! this exchange either: a caller either has state or does not.
//!
//! # Whole is the provider's to judge
//!
//! No identity rides this request, so no size and hash stand behind
//! the bytes. They are the provider's own state in the provider's own
//! format, and a provider validates what it is handed by that format
//! — a truncated continuation fails to open, which is the provider's
//! error to report, as a request error, with nothing run.
//!
//! # This endpoint's own, not [`shared`](crate::shared)
//!
//! Nothing but an agentic loop resumes from a continuation, so this
//! lives where it is used — a shape moves to `shared` when a second
//! endpoint needs it, not before.

mod request;

pub use request::*;

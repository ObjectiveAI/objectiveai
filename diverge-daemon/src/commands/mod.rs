//! Every request a requester puts to the daemon, kept in its own book.
//!
//! [`record`] is called once per scope request, at the one place every
//! request passes, and keeps the request as
//! [`command_log`](diverge_sdk::daemon::command_log) states: its path,
//! its body with secrets replaced ([`body`]), and whether the daemon
//! let it through — told by the scope's
//! [`Verdict`](diverge_sdk::wire::server::verdict::Verdict), the first
//! answer compared with the request's own `Forbidden`, or the handler's
//! word that it let the request through before a slow answer. The book
//! is the requester's ([`Commander`]): an account's, an
//! administrator's, or the tool's own log for a tool's container.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod commander;
mod record;
mod redact;

pub use commander::*;
pub use record::*;
pub use redact::*;

//! A requester's command log: every request it put to the daemon.
//!
//! Three kinds of requester put requests to the daemon: an account, an
//! administrator the configuration names, and a tool's container over
//! its own daemon connection. The daemon keeps each request as it
//! decides it — one [`Command`] — in the requester's own book: an
//! account's and an administrator's book hold commands alone, read as
//! [`logs`](crate::daemon::endpoints::logs) subjects of their own; a
//! tool's commands go into its [`tool_log`](crate::daemon::tool_log),
//! beside the calls it was asked.
//!
//! The rules:
//!
//! 1. One entry per scope request. The channels opened inside a scope —
//!    a cancel, a disconnect, a connect's MCP exchanges, which the tool
//!    log keeps — are not commands.
//! 2. `request` is the request's body as sent, `relay` and all, with
//!    every value a secret could be in replaced: a vault insert's
//!    `value`.
//! 3. `allowed` is `false` exactly when the daemon answered the request
//!    with its `Forbidden`; a request let through that then failed — not
//!    found, an error — was allowed. What the request came to is not
//!    kept.
//! 4. The entry is kept when the daemon decides: at the first answer, or
//!    — for a request whose answer is long in coming — the moment it is
//!    let through.
//! 5. A request the daemon could not read is not kept.
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod command;
mod item;
mod item_type;
mod item_wrapper;

pub use command::*;
pub use item::*;
pub use item_type::*;
pub use item_wrapper::*;

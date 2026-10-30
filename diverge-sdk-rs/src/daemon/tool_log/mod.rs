//! A tool's log: every exchange the daemon routed into the tool, kept.
//!
//! The daemon routes every MCP call and resource read a tool is asked —
//! by an agent's container, or by a client over a
//! [`connect`](crate::daemon::endpoints::standalone_tools::connect) —
//! and keeps each as it routes it: the ask, with who asked, then its
//! answer, which names the ask by its index. It keeps when the tool
//! began to have an ask in flight and when it ceased to, every
//! notification the tool sent, and the errors the daemon met keeping
//! the tool. The log is the daemon's own word, never the tool's, and
//! outlives every run; it is removed with the tool.
//!
//! Lists — `tools/list` and `resources/list` — are neither kept nor
//! counted in flight: every loop asks them, and they say nothing of
//! what the tool did.
//!
//! [`ItemWrapper`] is one entry, [`Item`] what it holds, and
//! [`ItemType`] the kinds by name. Both families read it the same way:
//! [`standalone_tools::logs`](crate::daemon::endpoints::standalone_tools::logs)
//! and [`dependency_tools::logs`](crate::daemon::endpoints::dependency_tools::logs).
//!
//! Its own files are flattened into it, so everything is named through
//! this module and not through the file it lives in.

mod call;
mod call_error;
mod call_result;
mod error;
mod item;
mod item_type;
mod item_wrapper;
mod notification;
mod read;
mod read_error;
mod read_result;

pub use call::*;
pub use call_error::*;
pub use call_result::*;
pub use error::*;
pub use item::*;
pub use item_type::*;
pub use item_wrapper::*;
pub use notification::*;
pub use read::*;
pub use read_error::*;
pub use read_result::*;

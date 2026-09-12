//! `/tool/notifications`: what the container's server says on its own
//! account — the shared
//! [`mcp::notifications`](crate::shared::mcp::notifications)
//! re-exported, a [`response::Frame`] per notification for the
//! connection's life, and, behind the `server` feature, the executor
//! that subscribes. Nothing is sent: the opening is the subscription.

pub use crate::shared::mcp::notifications::*;

#[cfg(feature = "server")]
pub mod execute;

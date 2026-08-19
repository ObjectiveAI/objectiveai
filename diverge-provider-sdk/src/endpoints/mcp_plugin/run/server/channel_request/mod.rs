//! The channels a server opens on a caller for an MCP plugin.
//!
//! Three, and they are the same ask in different clothes: something
//! the provider cannot reach. One fetches the image, one reaches the
//! database, one runs a command — see [`Frame`].
//!
//! What an OCI one carries is
//! [`http::request`](crate::shared::http::request), which is not here
//! and should not be: a tunneled HTTP request is the same request
//! whatever protocol rides it.
//!
//! [`Postgres`] is here, because a connection id is this exchange's
//! own and means nothing outside it. It is also the only one of the
//! three that is HALF an exchange — the caller opens the other half —
//! and that file is where the reason lives.

mod frame;
mod postgres;

pub use frame::*;
pub use postgres::*;

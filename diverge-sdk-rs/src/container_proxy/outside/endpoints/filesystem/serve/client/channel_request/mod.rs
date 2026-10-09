//! The channels the server opens on the proxy during a serve: the
//! nine asks, the stop, and the tree. See [`Frame`].
//!
//! The asks are the proxy's own frames — the ones a mount opens on
//! its scope, going the other way — and the frame is the
//! [`volumes::serve`](crate::provider::endpoints::volumes::serve) scope's
//! very frame, so a provider relaying a container serve forwards each
//! ask as it is: [`Path`], [`Read`], [`Write`], [`Rename`],
//! [`Truncate`] and [`Setattr`] are the mount's.

mod frame;

pub use frame::*;
pub use crate::container_proxy::outside::endpoints::fuse::mount::server::channel_request::{Path, Read, Rename, Setattr, Truncate, Write};

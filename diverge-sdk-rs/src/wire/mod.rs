//! The wire: what every protocol in this crate is spoken over.
//!
//! One WebSocket, every message one BINARY frame with a nine-byte
//! header — [`frame`]; the socket either end may hold, accepted or
//! dialled — [`connection`]; how a type becomes a payload's bytes and
//! comes back out — [`encode`] and [`decode`]; and the two frame-level
//! halves, [`client`], which mints scopes and channels and routes what
//! answers them, and [`server`], which is told about scopes and
//! answers inside them. None of it names an endpoint: the
//! [`provider`](crate::provider), the [`daemon`](crate::daemon) and
//! the [`container proxy`](crate::container_proxy) each define their
//! own request vocabulary on top of this and nothing here knows which
//! it is carrying.
//!
//! [`connection`] carries either kind of socket under either half,
//! because which end dialled is a fact about TCP and not about a
//! protocol: a provider usually waits to be dialled and sometimes
//! dials a caller it cannot otherwise reach, and a caller can as well
//! be dialled into. The frames are the same frames whichever way
//! round it went.

pub mod client;
pub mod connection;
pub mod decode;
pub mod encode;
pub mod frame;
pub mod server;

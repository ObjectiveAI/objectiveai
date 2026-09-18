//! What the three container scopes' executors share — and what the
//! provider's server borrows of it to speak the proxy's wire.
//!
//! Behind the `client` feature in full. Behind `server` alone, the
//! family-agnostic part: [`Answered`] and its readers, and the error
//! types, which the executors under
//! [`container_proxy_endpoints`](crate::container_proxy_endpoints)
//! use as the caller's do — the server is the client on that wire.
//! The asks, the answerers' dispatch, the family encoders, the
//! running scope and its pending writes are the caller's alone.
//!
//! Every server-opened ask's request and answer is a `shared` type,
//! and every client-opened channel's answer is either shared or one
//! of three byte-identical envelopes, so the machinery that reads and
//! answers them is written once here and each scope's `execute`
//! wraps it:
//!
//! - [`Ask`], the twenty-four asks a run scope's provider makes, owned,
//!   and the answer to each through the caller's
//!   [`Answerers`](crate::client::Answerers) on a task of its own,
//!   read off the scope by one serving loop.
//! - [`Answered`], what one client-opened channel's frames decode to,
//!   and the two readers over it: [`ChannelStream`] for a channel
//!   that streams, [`unary`] for one that answers once.
//! - [`Scoped`], the running scope every `ExecuteHandle` wraps: its
//!   handle and number, its pending [`Writes`], and the main stream's
//!   end, which every handle's `wait` reports.
//! - [`Encoders`], the three frames a family encodes for the shared
//!   machinery, since each family's channel frames are its own type.

pub mod answered;
pub mod streams;

mod open_error;
mod unary;

#[cfg(feature = "client")]
mod scoped;
#[cfg(feature = "client")]
mod writes;

#[cfg(feature = "client")]
mod ask;
#[cfg(feature = "client")]
mod encoders;

#[cfg(feature = "client")]
pub(crate) mod answer;
#[cfg(feature = "client")]
pub(crate) mod serve;

pub use answered::Answered;
#[cfg(feature = "client")]
pub use ask::*;
#[cfg(feature = "client")]
pub use encoders::*;
pub use open_error::*;
#[cfg(feature = "client")]
pub use scoped::*;
pub use streams::*;
pub use unary::*;
#[cfg(feature = "client")]
pub use writes::*;

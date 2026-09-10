//! What the three container scopes' executors share, behind the
//! `client` feature.
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

mod ask;
mod encoders;
mod open_error;
mod scoped;
mod unary;
mod writes;

pub(crate) mod answer;
pub(crate) mod serve;

pub use answered::Answered;
pub use ask::*;
pub use encoders::*;
pub use open_error::*;
pub use scoped::*;
pub use streams::*;
pub use unary::*;
pub use writes::*;

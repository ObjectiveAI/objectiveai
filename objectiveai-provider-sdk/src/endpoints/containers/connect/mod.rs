//! Joining a container somebody else created.
//!
//! Split by who SENDS: [`client`] is the connector's traffic,
//! [`server`] the provider's.
//!
//! # Three parties, and the provider is the middle one
//!
//! A connector names a container and offers an authorization. The
//! provider does not judge it — it relays it to whoever holds that
//! container's creation scope, as an
//! [`Authorize`](crate::endpoints::containers::create::server::channel_request::Frame::Authorize),
//! and waits. The creator answers yes or no, and that answer is
//! whether this scope opens at all.
//!
//! So the check happens somewhere neither end of THIS connection can
//! see, which is the point: a provider hosting a container does not
//! have to know what makes one connector acceptable and another not.
//! Only the creator knows, and only the creator is asked.

pub mod client;
pub mod server;

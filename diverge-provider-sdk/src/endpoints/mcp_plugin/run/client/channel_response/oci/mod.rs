//! The registry answers a caller sends back.
//!
//! What rides this channel is the OCI Distribution Specification —
//! the pull half of it,
//! exactly as a
//! [`laboratory run`](crate::endpoints::laboratories::run::client::channel_response::oci)
//! uses it, and for the same reasons. A provider's container runtime issues
//! ordinary registry requests, the provider relays them, and a
//! caller's own registry answers. Nothing on either side is aware of
//! the other.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else, and it is bytes. A registry answer is
//! HTTP and stays HTTP; this crate reads none of it, which is what
//! lets a caller behave like a proxy — rewriting a `Location` its
//! runtime could not reach, answering as an open registry while using
//! its own credentials upstream. See
//! [`shared::oci`](crate::shared::oci) for why that is the shape.

mod frame;

pub use frame::*;

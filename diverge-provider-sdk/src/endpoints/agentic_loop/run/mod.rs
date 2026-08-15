//! Running an agent through its turns.
//!
//! Split by who SENDS: [`client`] is everything a client puts on the
//! wire, [`server`] everything a provider does.
//!
//! The one scope [`agentic_loop`](super) has, and the reason it is
//! named rather than being the endpoint itself: every other endpoint's
//! scopes are named — a [`laboratory`](crate::endpoints::laboratories)
//! is run or connected to, a
//! [`volume`](crate::endpoints::volumes) is listed, watched, made,
//! edited or destroyed — and an endpoint with exactly one scope is not
//! a reason to say it differently.

pub mod client;
pub mod server;

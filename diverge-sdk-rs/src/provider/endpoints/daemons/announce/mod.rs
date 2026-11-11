//! Announcing the caller to a provider's daemons list, and keeping it
//! there.
//!
//! One request carrying one line of text, the announcement, and a
//! scope kept open, shaped as
//! [`daemons::listen`](crate::provider::endpoints::daemons::listen):
//! while the scope is open, the caller's identity is in the provider's
//! [`daemons::list`](crate::provider::endpoints::daemons::list) with
//! that announcement, and when the scope ends it leaves. Nothing is
//! answered while the announce stands. A connection holds at most one
//! announce open.
//!
//! Split by who SENDS, as everywhere else: the announcement and the
//! cancel are in [`client`], the one refusal in [`server`].

pub mod client;
pub mod server;

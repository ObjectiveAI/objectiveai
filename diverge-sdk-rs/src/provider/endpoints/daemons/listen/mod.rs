//! Listening for the messages sent to the caller, and keeping the
//! scope open.
//!
//! One question with no parameters, and a stream kept open, shaped as
//! [`daemons::list`](crate::provider::endpoints::daemons::list): every message a
//! [`message`](super::message) names the caller's identity in, one response
//! each, in the order the provider queued them, until the caller
//! cancels. A connection holds at most one listen scope open, and an
//! identity at most one on the provider as a whole.
//!
//! Split by who SENDS, as everywhere else: the question and the cancel
//! are in [`client`], the messages in [`server`].

pub mod client;
pub mod server;

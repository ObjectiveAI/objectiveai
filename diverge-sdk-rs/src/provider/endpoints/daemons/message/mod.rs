//! Sending one message to one identity, and learning its fate.
//!
//! One question and one answer, as
//! [`daemons::connect`](crate::provider::endpoints::daemons::connect)
//! is a request with a body: the receiver's identity and the message.
//! The answer comes when the fate is known — the receiver's reader
//! took the message, the receiver is not listening, or the sender is
//! over its rate — and the scope finishes after it. The caller may
//! cancel while the message waits; a cancel in time ends the scope with
//! the finish and nothing before it. A connection holds at most one
//! message open. Any connected identity may send, announced or not and
//! listening or not, and the provider's rate limit applies to every
//! send.
//!
//! Split by who SENDS, as everywhere else: the question and the cancel
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;

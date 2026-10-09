//! The server side of a serve: what the proxy sends.
//!
//! [`response`] is what comes back on channel `0`: that the subtree
//! is served, or why not, and the finish when the server stops.
//! [`channel_response`] answers each ask the server opens, with that
//! ask's own frame from the shared vocabulary, and each tree the
//! server opens with a filetree stream.
//!
//! There is no `channel_request`, because the proxy opens no channel
//! on a serve. It answers.

pub mod channel_response;
pub mod response;

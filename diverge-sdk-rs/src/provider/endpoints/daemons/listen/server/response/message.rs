//! One message, as its receiver reads it.

use bytes::Bytes;

use crate::shared::Identity;

/// A message sent to the caller: who sent it, and what it says.
///
/// The body is bytes and nothing else: the provider neither reads nor
/// changes it, and prescribes no format for it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Message {
    /// The identity of the sending connection, `(iss, sub)`, as its
    /// authorization established it.
    pub from: Identity,
    /// The body, exactly as the send carried it.
    pub body: Bytes,
}

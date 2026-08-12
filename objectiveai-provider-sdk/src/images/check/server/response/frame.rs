//! What a server's response frame carries in an image check.

use super::Response;

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response)
/// on channel `0` of an image check.
///
/// # A frame type is never serialized
///
/// This enum is not a wire shape and carries no serde derives. It is
/// the DISPATCH layer: it says which payload a response frame holds,
/// and that is settled by the scope the frame arrives in rather than
/// by anything in its bytes. What goes on the wire is the payload's
/// own JSON, with nothing wrapped around it.
///
/// Which is why the payloads carry the derives and this does not. A
/// frame type that could be serialized would be a frame type that had
/// started describing the wire twice.
///
/// # One variant
///
/// A check is one question and one reply, so there is exactly one of
/// these per scope, between the ack that mints it and the finish that
/// ends it. An enum anyway, for the same reason the agentic loop's is
/// one: a second thing a server might stream on channel `0` of a
/// check would be a new variant rather than a changed shape.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The answer to the check.
    Check(Response),
}

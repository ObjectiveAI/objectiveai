//! What a server's response frame carries in an image check.

use super::Response;

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response)
/// on channel `0` of an image check.
///
/// A check is one question and one reply, so there is exactly one of
/// these per scope, between the ack that mints it and the finish that
/// ends it.
///
/// # A frame type is never serialized
///
/// This type is not a wire shape and carries no serde derives. It is
/// the DISPATCH layer: it says which payload a response frame holds,
/// and that is settled by the scope the frame arrives in rather than
/// by anything in its bytes. What goes on the wire is the payload's
/// own JSON, with nothing wrapped around it.
///
/// Which is why the payload carries the derives and this does not. A
/// frame type that could be serialized would be a frame type that had
/// started describing the wire twice.
///
/// # A struct, not an enum
///
/// There is one thing it can hold, so there is nothing to choose
/// between — the same reason
/// [`postgres::Frame`](crate::agentic_loop::client::response::postgres::Frame)
/// is one. A single-variant enum would ask every reader to match on a
/// decision that has already been made.
///
/// If a second thing to stream here ever appears, this becomes an enum
/// then. That is a breaking change, and the honest one: carrying a
/// variant nobody uses against a need that may never come is a cost
/// paid every day for a day that may not arrive.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// The answer to the check.
    pub Response,
);

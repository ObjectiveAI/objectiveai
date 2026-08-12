//! What a server's response frame carries in an image check.

use serde::{Deserialize, Serialize};

use super::{Available, Unavailable};

/// The payload of a [`ServerFrame::Response`](crate::frame::server::ServerFrame::Response)
/// on channel `0` of an image check.
///
/// Whether a provider can supply the image that was asked about — and
/// the whole of the answer, since a check is one question and one
/// reply. There is exactly one of these per scope, between the ack
/// that mints it and the finish that ends it.
///
/// It is called `Frame` for the same reason the agentic loop's is: the
/// payload of a response frame is whatever the scope's protocol says
/// it is, and naming each scope's uniformly is what lets a reader
/// dispatch on the scope and decode without a special case. Here that
/// payload is the answer itself, so there is no wrapper around it —
/// a one-variant enum holding a `Response` would be a layer that says
/// nothing the module has not said.
///
/// Untagged, with each variant's payload carrying its own `type`
/// constant — the same discipline the agentic loop chunks use. serde
/// has no tag of its own to read, so the answer goes on the wire as
/// itself rather than as a wrapper around itself.
///
/// Two variants rather than a `bool`, because only one of the two
/// answers has anything more to say. An available image has terms
/// attached to it; an unavailable one is just absent. A boolean would
/// force everything that qualifies availability to sit beside it as an
/// optional field that is meaningless when the answer is no.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Frame {
    /// The provider can supply it.
    Available(Available),
    /// It cannot.
    Unavailable(Unavailable),
}

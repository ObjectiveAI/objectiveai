//! A request that owns what it carries.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{Method, Request};

/// A [`Request`] with nothing borrowed.
///
/// The same four fields, with the body owned rather than pointing into
/// the frame it was read out of.
///
/// # Why it exists
///
/// Because [`Request`] cannot be handed over. Its
/// [`body`](Request::body) is a `&RawValue` borrowed from whatever
/// buffer it was decoded from, which is right for the thing it is —
/// a request read out of a frame and encoded into another without the
/// bytes being copied or reparsed on the way.
///
/// It is wrong for anything that has to OUTLIVE that buffer. A request
/// arriving on a stream is the case: the bytes it was parsed from
/// belong to whatever parsed it, and the item has to stand on its own
/// once yielded.
///
/// So this is what a producer yields and [`Request`] is what a wire
/// takes, and [`request`](Self::request) is the step between.
///
/// # It is not a second wire form
///
/// It serializes identically — same field names, same omissions — so a
/// document written from one reads back as the other. That is a
/// property worth keeping rather than a use worth having: what goes on
/// a wire is [`Request`], and this exists so that something can hold
/// one until it does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owned {
    /// See [`Request::method`].
    pub method: Method,
    /// See [`Request::path`].
    pub path: String,
    /// See [`Request::headers`].
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
    /// See [`Request::body`]. Owned here, borrowed there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Box<RawValue>>,
}

/// By what it says, not by how the body was spelled — the same
/// comparison [`Request`] makes, and for the same reason.
impl PartialEq for Owned {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method
            && self.path == other.path
            && self.headers == other.headers
            && self.body.as_deref().map(RawValue::get)
                == other.body.as_deref().map(RawValue::get)
    }
}

impl Owned {
    /// Borrow this as the form a wire takes.
    ///
    /// # It copies the path and the headers
    ///
    /// Because [`Request`] owns those two and borrows only the body, so
    /// there is nothing to lend them from. The copy is a string and a
    /// small map; the body, which is the part that can be large, is
    /// borrowed and not copied at all.
    ///
    /// Which is the shape worth having. A request is turned into this
    /// once, when it arrives, and encoded from it once, when it is
    /// relayed — and the bytes nobody wants to touch are touched by
    /// neither step.
    pub fn request(&self) -> Request<'_> {
        Request {
            method: self.method,
            path: self.path.clone(),
            headers: self.headers.clone(),
            body: self.body.as_deref(),
        }
    }
}

impl From<&Request<'_>> for Owned {
    fn from(request: &Request<'_>) -> Self {
        Owned {
            method: request.method,
            path: request.path.clone(),
            headers: request.headers.clone(),
            body: request.body.map(|body| body.to_owned()),
        }
    }
}

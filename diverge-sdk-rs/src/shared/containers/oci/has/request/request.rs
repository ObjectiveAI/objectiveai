//! Asking whether an image is held.

use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::shared::containers::request::Image;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The image, as the run request named it — its digest, and the
/// references under which its bytes may be fetched — and deliberately
/// nothing else: the name the caller holds the image under is the
/// caller's to answer.
///
/// # On the wire
///
/// Exactly the [`Image`]'s JSON, with nothing around it:
///
/// ```json
/// {"digest":"sha256:<hex>","references":[{"registry":"…","name":"…"}]}
/// ```
///
/// `references` is absent when there are none. The same value is the
/// argument of the provider endpoint
/// [`images::oci`](crate::provider::endpoints::images::oci), so an
/// image asked about anywhere is named the same way everywhere.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Request {
    /// The image asked about.
    pub image: Image,
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}

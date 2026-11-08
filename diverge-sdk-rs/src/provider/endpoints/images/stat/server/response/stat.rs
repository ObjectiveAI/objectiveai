//! What a stat says about an image.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::images::list::server::response::Image;

/// What a listing says about an image, and what it was built from.
///
/// # Nested rather than flattened
///
/// The listing's [`Image`] sits inside this as a field rather than
/// having its fields copied in, so there is one definition of what a
/// listing says. Postcard writes a nested struct as its fields in
/// place, so the wire reads `digest`, `description`, `bytes`,
/// `created`, then the one field of its own — a listing's record with
/// one field appended.
///
/// # Why the Containerfile is not in the listing
///
/// Because it is as long as its author made it, and a listing that
/// carried every image's Containerfile would carry the ones nobody
/// asked about. A caller pays for it here, one image at a time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stat {
    /// The image as a listing reports it: its digest, its
    /// description, its size, and when the caller came to own it.
    pub image: Image,
    /// The Containerfile the image was built from, verbatim, as the
    /// [`build`](crate::provider::endpoints::images::build) request
    /// carried it; `None` for an image the caller
    /// [pushed](crate::provider::endpoints::images::push), which was
    /// built nowhere the provider saw. One postcard byte, `0` for none
    /// and `1` followed by the text.
    pub containerfile: Option<String>,
}

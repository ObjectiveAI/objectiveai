//! Where the provider took an image from.

use serde::{Deserialize, Serialize};

/// Where an image a provider took came from: a registry it uses, the
/// caller, or the caller's own Store.
///
/// Carried by every answer that says an image was taken: a run's
/// [`Id`](super::Id), and an
/// [`images::push`](crate::provider::endpoints::images::push)'s
/// `Pushed`.
///
/// # The order a run's image is looked for in
///
/// 1. **The caller's Store.** A digest the caller owns in its Store —
///    one it built or pushed on this provider, and has not deleted —
///    is run from there: [`Store`](Self::Store). No nonce check: the
///    caller proved its ownership when it gained it.
/// 2. **A registry the provider uses.** A reference the run names on a
///    registry the provider's policy lists, or an image the provider
///    holds from such a registry already:
///    [`Registry`](Self::Registry). No nonce check: the bytes are the
///    registry's, under the digest, and the caller supplied nothing.
/// 3. **The caller.** A caller that answers the
///    [`has`](crate::shared::containers::oci::has) question: the
///    provider pulls what it lacks, then makes the nonce check of
///    [`prove`](crate::shared::containers::oci::prove), always, and
///    only a pass is [`Client`](Self::Client); a failure is the run's
///    `ProofFailed`.
/// 4. **Otherwise** the image cannot be supplied, and the run fails.
///
/// So a registry is preferred to the caller, and the caller's Store to
/// both. A push consults only the second and the third, since a digest
/// already in the caller's Store is not pushed again.
///
/// # On the wire
///
/// Externally tagged, `snake_case`: `{"registry":{"reference":"…"}}`,
/// `"client"` or `"store"` in JSON, and the variant's index then its
/// fields in postcard.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A registry the provider uses served the image.
    Registry {
        /// The reference the provider took it by,
        /// `<registry>/<name>@<digest>`.
        reference: String,
    },
    /// The caller supplied the image, and proved that it holds it.
    Client,
    /// The caller's own Store held the image. A run's answer only: a
    /// push of a digest the caller owns is `Exists`.
    Store,
}

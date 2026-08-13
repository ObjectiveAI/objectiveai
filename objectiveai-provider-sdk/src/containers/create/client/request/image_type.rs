//! Who produces a container's image.

use serde::{Deserialize, Serialize};

/// Who is responsible for producing the image.
///
/// One question — can the provider get these bytes, and if not, who
/// can — and it is the only thing that varies. All three name what
/// they want with a reference; they differ in who is asked for it.
///
/// It also decides how
/// [`image_reference`](super::Frame::image_reference) is read, which
/// is why the two fields belong together and neither means much
/// alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    /// The caller serves it.
    ///
    /// For images that exist nowhere a provider can reach — built
    /// locally, never pushed, carrying a digest no registry has heard
    /// of.
    ///
    /// The caller runs a registry and the provider relays to it. A
    /// request the container runtime makes arrives at `/v2/<scope>/…`,
    /// the provider strips the scope segment, and the rest goes to the
    /// caller as an
    /// [`http::request::Request`](crate::http::request::Request) on a
    /// channel. The runtime never learns it is talking to a proxy; the
    /// caller never learns it is not being pulled from directly.
    ///
    /// Which is why a provider needs nothing else. It does not read a
    /// manifest, diff layer digests, or decide what a blob is — a
    /// runtime already indexes layers by compressed digest and already
    /// skips the ones it holds, so letting it pull means that logic is
    /// USED rather than reimplemented beside it. One cache, and no
    /// second one to disagree with it. `Range` resumes and `HEAD`
    /// probes work for the same reason: they are headers, and headers
    /// cross.
    Client,
    /// The provider produces it, however it likes.
    ///
    /// Its own mirror, a pull-through cache, a private registry it
    /// holds credentials for, or something already on disk. A caller
    /// does not know and is not told.
    ///
    /// Which is what makes proprietary images expressible: a provider
    /// serves an image no public registry carries, and a caller asks
    /// for it, without the caller ever being able to fetch it itself.
    /// A host written into the reference is the provider's to honour
    /// or ignore — it resolves by its own rules, and a caller naming a
    /// source it cannot reach is asserting something it has no
    /// standing to assert.
    ///
    /// Ask [`images::check`](crate::images::check) first if the answer
    /// matters before the container does.
    Server,
    /// The provider pulls from where the caller says.
    ///
    /// The one case where the CALLER chooses the source, for public
    /// images where it knows what it wants and the provider has no
    /// opinion.
    ///
    /// Which makes the reference a host a caller picked, and the
    /// provider connects there and runs what it finds. Which registries
    /// are reachable is a provider's policy to set and enforce, and
    /// nothing here can express that policy — a caller learns it by
    /// being refused.
    Registry,
}

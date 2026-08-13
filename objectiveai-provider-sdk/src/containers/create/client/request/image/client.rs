//! The caller supplies the image.

use serde::{Deserialize, Serialize};

/// The caller serves the image itself.
///
/// For images that exist nowhere a provider can reach — built
/// locally, never pushed, and carrying a digest no registry has ever
/// heard of.
///
/// # How it works
///
/// The caller runs a registry. The provider serves a registry endpoint
/// of its own, points its container runtime at it, and relays: a
/// request the runtime makes arrives at `/v2/<scope>/…`, the provider
/// strips the scope segment, and what is left goes to the caller as an
/// [`http::request::Request`](crate::http::request::Request) on a
/// channel.
///
/// The runtime never learns it is talking to a proxy. The caller never
/// learns it is not being pulled from directly.
///
/// # Which is why there is nothing here but a name
///
/// The provider does not read a manifest, does not diff layer digests
/// against a store of its own, and does not decide what a blob is.
/// A container runtime already indexes layers by compressed digest and
/// already skips the ones it holds, so letting it do the pulling means
/// that logic is USED rather than reimplemented beside it — one cache,
/// and no second one to disagree with it.
///
/// What falls out is the dedup a caller wants without anyone asking
/// for it: three variants of one base image send the base once, and an
/// image whose base the provider pulled last week for somebody else
/// sends only the layers that are new. `Range` resumes and `HEAD`
/// probes work for the same reason — they are headers, and headers
/// cross.
///
/// # Refusing early
///
/// A provider that wants to check size before pulling asks for the
/// manifest through its own proxy and sums the descriptors. That is
/// one request, and it is better than a caller declaring the number:
/// what the caller serves is what gets pulled, so the check is made
/// against the thing itself rather than against a claim about it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Client {
    /// The discriminator.
    pub r#type: ClientType,
    /// What to ask the caller's registry for — `myimage:latest`,
    /// `myimage@sha256:…`.
    ///
    /// A repository and a reference, with no host: the host is the
    /// caller, which is where this arrived from and is not something
    /// it has to say.
    ///
    /// The provider concatenates rather than parses. `localhost:PORT/`
    /// plus the scope plus this is a reference a runtime can pull,
    /// because a tag or digest suffix stays on the end where one
    /// belongs.
    ///
    /// # It lands in a URL path
    ///
    /// Which makes it a path fragment wearing the costume of a name.
    /// A `..` in it walks out of the scope segment and into another
    /// caller's namespace, so a provider normalizes or refuses before
    /// concatenating. The field cannot enforce that and does not
    /// pretend to.
    pub reference: String,
}

/// [`Client`]'s discriminator.
///
/// One variant, and part of why [`Image`](super::Image) can be
/// untagged: no other source can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ClientType {
    /// Always this.
    #[serde(rename = "client")]
    #[default]
    Client,
}

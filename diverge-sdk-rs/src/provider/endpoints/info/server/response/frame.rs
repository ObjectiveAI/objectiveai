//! What a server's response frame carries for an info request.

use serde::{Deserialize, Serialize};

use super::Capabilities;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What the provider says it is: the revision of this specification
/// it implements, and the capabilities it holds out.
///
/// One of these on channel `0`, then the scope finishes. The payload
/// is this struct as JSON — no tag, because there is nothing to
/// discriminate; it runs to the end, so it needs no length either.
///
/// ```json
/// {"version":"2.3.0","capabilities":{"images_store":true,
/// "images_registries":["docker.io"],"volumes_persistent":{"mount":true,
/// "fuse_serve":true,"mount_many":true,"fuse_serve_many":false,
/// "mount_and_fuse_serve":false}}}
/// ```
///
/// # The version is fixed, and the specification defines it
///
/// The revision of the specification the provider implements, which
/// is this crate's version: `MAJOR.MINOR.PATCH`, the string every
/// page of the specification prints as its revision. The handler
/// sends the crate's own, read from the manifest at compile time.
/// Nothing else is a conforming answer — not a build hash, not a
/// name, and not the empty string. A caller reads it as the first
/// fact it needs before composing anything else: which specification
/// the far end speaks.
///
/// # The capabilities are the provider's, and it states them
///
/// What of the specification this provider does — see
/// [`Capabilities`] for each, and the variant of each endpoint's
/// response that a request past them is answered with. A caller
/// reads them as the second fact: which requests to compose at all.
///
/// # There is no failure
///
/// As with [`capacity`](crate::provider::endpoints::capacity), and no
/// other response in this specification. Every other scope can come
/// back with the provider saying it could not — an image it cannot
/// supply, a container that would not start — because every other
/// scope asks it to DO something.
///
/// This asks it to say what it is, which it always knows. A provider
/// that could not answer this could not have received the question.
///
/// Which is also why there is nothing to tag. A tag tells two things
/// apart, and there are not two things.
///
/// # A newer provider is read, not refused
///
/// A member this version does not know is ignored, here and in
/// [`Capabilities`]: a provider that states a capability a caller
/// does not know is a provider the caller can still use for
/// everything it does know, and a caller that refused the answer
/// could use it for nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The revision: `MAJOR.MINOR.PATCH`.
    pub version: String,
    /// What the provider does. Absent, nothing: a provider that holds
    /// out no capability is one that serves only what every provider
    /// serves.
    #[serde(default)]
    pub capabilities: Capabilities,
}

/// The struct as JSON, and nothing in front of it.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// One way to fail: bytes that are not this struct's JSON. What
    /// the version says is the caller's to judge against the revision
    /// it expects.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

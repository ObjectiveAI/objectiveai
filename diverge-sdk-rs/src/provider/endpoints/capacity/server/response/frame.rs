//! What a server's response frame carries for a capacity request.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// How much the caller may still use: four counts of bytes, each the
/// most of its resource the caller could use up as of the instant of
/// the answer, as the provider allots it.
///
/// One of these on channel `0`, then the scope finishes. The payload
/// is this struct as JSON — no tag, because there is nothing to
/// discriminate; it runs to the end, so it needs no length either.
///
/// ```json
/// {"volumes_disk":107374182400,"images_disk":53687091200,
/// "container_memory":4294967296,"container_disk":17179869184}
/// ```
///
/// # Each number is the caller's, as of now
///
/// What THIS caller — the connection's identity — could use up at the
/// instant the provider answered, as the provider allots it: a
/// provider that lets every caller use everything it holds answers
/// what it holds; one that partitions its callers answers the
/// caller's share; neither answers more than it would, at that
/// instant, let the caller have. Nothing is set aside by answering: a
/// request that relies on a number — a create, a build, a run — is
/// answered on its own terms, and may be refused for want of room the
/// answer reported.
///
/// # Reckoned by reservation
///
/// The two container numbers count against what the provider's
/// running containers RESERVED — the `memory` and `disk` their
/// requests stated — and not against what the host has free: a run
/// whose `memory` and `disk` are within them is one the provider's
/// caps admit.
///
/// # Every member is sent; a caller reads what the capabilities allow
///
/// A member whose resource the provider's
/// [`Capabilities`](crate::provider::endpoints::info::server::response::Capabilities)
/// do not hold out carries any number, and a caller does not read it:
/// `volumes_disk` where no volume mode object is held out,
/// `images_disk` where `images_store` is not. The container members
/// are always read.
///
/// # There is no failure
///
/// As with [`info`](crate::provider::endpoints::info): every
/// provider counts what it holds, so there is nothing to refuse and
/// nothing to tag. A member this version does not know is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The largest size, in BYTES, a new volume of the caller could
    /// have: room for a volume, which lives in one place, and not the
    /// sum of the provider's room. Read only where a volume mode
    /// object is held out.
    pub volumes_disk: u64,
    /// The number of BYTES the caller could still add to the client
    /// image store by a build. Read only where `images_store` is held
    /// out.
    pub images_disk: u64,
    /// The number of BYTES of memory a container run of the caller
    /// could still reserve: what remains of the provider's memory cap
    /// after every running container's `memory`.
    pub container_memory: u64,
    /// The number of BYTES of overlay disk a container run of the
    /// caller could still reserve: what remains of the provider's
    /// overlay cap after every running container's `disk`, and
    /// whatever else the provider counts against the same cap.
    pub container_disk: u64,
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
    /// One way to fail: bytes that are not this struct's JSON.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}

//! The caller names a registry reference.

use serde::{Deserialize, Serialize};

/// The caller names a registry reference for the provider to pull.
///
/// For public images where the caller knows what it wants and the
/// provider has no opinion — the one variant where the CALLER chooses
/// the source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Registry {
    /// The discriminator.
    pub r#type: RegistryType,
    /// The reference, passed through as written.
    ///
    /// Whatever a container runtime accepts: `ubuntu:22.04`,
    /// `ghcr.io/org/image@sha256:…`, a bare name left to resolve
    /// against whatever default the provider uses.
    ///
    /// # Not pinned, deliberately
    ///
    /// Everything else in this API is digest-addressed so that the
    /// same request means the same bytes. This is not, and a tag here
    /// resolves differently next week — which is the point. A caller
    /// asking for `ubuntu:22.04` is asking to track it, exactly as a
    /// loose version constraint on a Python requirement is.
    ///
    /// A caller that wants the guarantee writes a digest into the
    /// reference and gets it. The choice is the caller's, and this
    /// field declines to make it for them.
    ///
    /// # A caller choosing the host is a provider's problem
    ///
    /// Passing this through means the provider connects where a
    /// caller pointed it and runs what it finds. Which registries are
    /// reachable is a provider's policy to set and enforce, and this
    /// field cannot express that policy — a caller learns it by being
    /// refused.
    pub reference: String,
}

/// [`Registry`]'s discriminator.
///
/// One variant, and part of why [`Image`](super::Image) can be
/// untagged: no other source can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum RegistryType {
    /// Always this.
    #[serde(rename = "registry")]
    #[default]
    Registry,
}

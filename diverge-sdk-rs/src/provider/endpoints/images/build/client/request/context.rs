//! The directory a build reads from.

use serde::{Deserialize, Serialize};

/// A directory of one of the caller's volumes, as the context and the
/// working directory of a build: the same two fields a
/// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
/// names its host side by, and nothing else.
///
/// # What the build does with it
///
/// Reads it. The Containerfile's `COPY` and `ADD` resolve against it,
/// and so does a `RUN --mount=type=bind` from the context; nothing a
/// build does writes into it, since a Containerfile copies in and not
/// out. So there is no mode here: a build takes the volume's shared
/// hold for its life, as a mount does, and a volume that cannot give
/// one — held exclusively, or a persistent volume held at all — is
/// the build refused.
///
/// # The same access model as a mount
///
/// The volume is one of the caller's listing, named by name, and the
/// path is components into it. A caller cannot escape upward, because
/// there is no component it can write that means "up" — `..` is a
/// name, not an instruction, and a provider descends names and never
/// resolves a path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Context {
    /// Which volume, by the name a listing gave it.
    pub volume_name: String,
    /// How far into that volume the context is, as path components
    /// relative to it. Empty is the volume itself.
    pub volume_relative_path: Vec<String>,
}

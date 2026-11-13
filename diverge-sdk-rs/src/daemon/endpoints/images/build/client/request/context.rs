//! The directory a build reads from.

use serde::{Deserialize, Serialize};

/// A directory of one volume on the build's provider, as the context
/// of the build: what the Containerfile's `COPY` and `ADD` resolve
/// against. The build reads it and writes nothing into it. It is the
/// provider's
/// [`Context`](crate::provider::endpoints::images::build::client::request::Context),
/// the volume named as the daemon lists it.
///
/// The volume is on the provider the request names, since a build
/// reads where it runs. The path is components into the volume: no
/// component means "up", so a context cannot reach outside its volume.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    /// The volume, by its name on the build's provider.
    pub volume: String,
    /// How far into the volume the context is, as path components.
    /// Absent or empty, the volume itself.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
}

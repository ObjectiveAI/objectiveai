//! One image the provider offers as its own.

use serde::{Deserialize, Serialize};

/// An image a caller may run by naming it as a `server` image: the
/// pair `images::check` answers for, and the pair a `server` deploy
/// is allowed to run. Nothing outside this list is either.
///
/// ```yaml
/// - name: acme/tools
///   digest: sha256:9f2c…
/// ```
///
/// The provider never pulls a listed image: it is in podman's storage
/// before the provider starts, under a name podman's short-name
/// resolution reaches from `name` — `localhost/<name>` for an image
/// built or loaded on this host, `<registry>/<name>` for one pulled
/// from a registry in podman's unqualified search list. An image held
/// under any other prefix is retagged to one of those. A run names
/// it as `<name>@<digest>` and podman looks no further than its own
/// storage, so a listed pair that is not there is a run that fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerImage {
    /// The repository path, with no host: `acme/tools`,
    /// `library/nginx`. What a caller's request carries as `name`.
    pub name: String,
    /// The manifest digest, `<algorithm>:<hex>`. What a caller's
    /// request carries as `digest`.
    pub digest: String,
}

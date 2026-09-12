//! A volume that exists before any client asks.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A volume that exists already, under a name the provider chose.
///
/// What a listing reports beside the name is read, not configured:
/// `bytes` from the filesystem the directory is on, and `created`
/// from the directory's birth time, or from the provider's own start
/// where the filesystem records none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixed {
    /// The name a listing gives it. Unique among the fixed volumes,
    /// and never given to a created volume of any identity: the name
    /// rule of `volumes::create` is enforced against it.
    pub name: String,
    /// An ABSOLUTE path to the directory that is the volume. A
    /// relative path is refused when the configuration is loaded.
    pub path: PathBuf,
    /// The hook, by name, that says which identities the volume is
    /// listed to: the folder `hooks/<name>/` of the provider's
    /// directory, run as [`hook`](crate::hook) provides. It reads a
    /// [`HookInput`] and writes a [`HookOutput`]. Absent means every
    /// identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorize_hook: Option<String>,
}

/// What the hook receives on stdin, as one line of JSON.
///
/// ```json
/// {"identity": "acme", "volume": "datasets"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookInput {
    /// The identity asking: the string the authorizer answered when
    /// the client connected.
    pub identity: String,
    /// The fixed volume's `name`, as the configuration wrote it.
    pub volume: String,
}

/// What the hook writes to stdout, as one JSON document, on exit `0`.
///
/// Anything else — a non-zero exit, a stdout that is not this — is a
/// [`hook::Error`](crate::hook::Error): the hook has not answered, and
/// the volume is not listed to that identity.
///
/// ```json
/// {"authorized": true}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookOutput {
    /// `true`, the volume is listed to the identity and may be
    /// mounted by it; `false`, it is not, and the identity is not
    /// told that it exists.
    pub authorized: bool,
}

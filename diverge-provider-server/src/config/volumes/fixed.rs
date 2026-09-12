//! A volume that exists before any client asks.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::super::Hook;

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
    /// The command that says which identities the volume is listed
    /// to. Absent means every identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorize: Option<Hook>,
}

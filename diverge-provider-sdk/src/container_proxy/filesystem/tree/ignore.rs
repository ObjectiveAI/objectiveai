//! What the proxy leaves out of the tree, and how the server says so.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The environment variable the server sets on the proxy: the paths
/// the tree must not contain, as an [`Ignore`] in JSON.
pub const IGNORE_ENV: &str = "DIVERGE_CONTAINER_PROXY_FILETREE_IGNORE";

/// Paths the filetree does not contain: never walked, never watched,
/// an event under one dropped.
///
/// The server names the MOUNTS here — the content it placed in the
/// container, read-only, that the caller already holds and whose
/// watch would cost the walk and yield nothing. `/proc`, `/sys` and
/// `/dev` are the proxy's own and are never listed. Each path is
/// components from the container's root, the shape every path in this
/// crate takes; an empty one is dropped rather than read as the root.
///
/// The variable's value is a JSON array of arrays of strings, which
/// is what serde makes of this newtype. A value that does not parse
/// is EMPTY, by rule: the proxy runs with the three and nothing more,
/// and the tree is larger than meant rather than absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ignore(pub Vec<Vec<String>>);

impl Ignore {
    /// Read the variable's value. Anything that is not the JSON above
    /// — an unset variable read as empty text, a stray word, a shape
    /// that is not arrays of strings — is the empty set.
    pub fn parse(value: &str) -> Self {
        serde_json::from_str(value).unwrap_or_default()
    }
}

/// The variable's value: the JSON the proxy parses.
impl fmt::Display for Ignore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}

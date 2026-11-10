//! Where a rotated secret is read back from, through the entry.

use serde::Serialize;

/// The two channels the entry can read for the harness: a setting of
/// the running runtime (`getSetting`), or an entry of Eliza's own
/// vault. The third channel, a file, the harness reads itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadKind {
    /// `runtime.getSetting(key)`.
    Setting,
    /// `@elizaos/vault`'s `get(key)`, under the master key the
    /// process environment carries.
    Vault,
}

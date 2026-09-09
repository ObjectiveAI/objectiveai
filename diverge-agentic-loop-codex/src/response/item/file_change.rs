//! The `file_change` item: a patch the agent applied.

use serde::Deserialize;

/// A set of file changes by the agent, completed once the patch
/// succeeds or fails.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileChange {
    /// Every file the patch touched.
    pub changes: Vec<FileUpdateChange>,
    /// Whether the patch applied.
    pub status: PatchApplyStatus,
}

/// One file of a patch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileUpdateChange {
    /// The file's path.
    pub path: String,
    /// What happened to it.
    pub kind: PatchChangeKind,
}

/// The status of a patch. The core's `declined` is folded into
/// [`Failed`](Self::Failed) on this wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchApplyStatus {
    /// Applying.
    InProgress,
    /// Applied.
    Completed,
    /// Did not apply — or was declined, which the wire spells the
    /// same.
    Failed,
}

/// What a patch did to a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchChangeKind {
    /// Created it.
    Add,
    /// Removed it.
    Delete,
    /// Changed it.
    Update,
}

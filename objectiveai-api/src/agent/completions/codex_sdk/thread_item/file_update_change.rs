use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileUpdateChange {
    pub path: String,
    pub kind: super::PatchChangeKind,
}

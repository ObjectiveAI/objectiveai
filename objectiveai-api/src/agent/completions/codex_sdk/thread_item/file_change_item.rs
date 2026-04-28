use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeItemType {
    FileChange,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChangeItem {
    pub id: String,
    pub r#type: FileChangeItemType,
    pub changes: Vec<super::FileUpdateChange>,
    pub status: super::PatchApplyStatus,
}

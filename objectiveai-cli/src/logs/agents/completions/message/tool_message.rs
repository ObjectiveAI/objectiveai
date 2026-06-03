//! Free-function port of `ToolMessage::extract`.

use objectiveai_sdk::agent::completions::message::{
    ToolMessage, ToolMessageLog,
};

use crate::filesystem::logs::LogFile;

/// Extract a `ToolMessage`'s content into per-leaf log files,
/// returning a [`ToolMessageLog`] (with `RichContentLog` in place of
/// `content`) plus the [`LogFile`]s the caller writes. `tool_call_id`
/// and `metadata` stay inline.
pub fn extract(
    msg: ToolMessage,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (ToolMessageLog, Vec<LogFile>) {
    let (content, files) = super::rich_content::extract_media(
        msg.content,
        &format!("{route_base}/messages"),
        id,
        message_index,
    );
    (
        ToolMessageLog {
            content,
            tool_call_id: msg.tool_call_id,
            metadata: msg.metadata,
        },
        files,
    )
}

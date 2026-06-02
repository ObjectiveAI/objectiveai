//! Free-function port of `UserMessage::extract`.

use objectiveai_sdk::agent::completions::message::{UserMessage, UserMessageLog};

use crate::filesystem::logs::LogFile;

/// Extract a `UserMessage`'s content into per-leaf log files,
/// returning a [`UserMessageLog`] (with `RichContentLog` in place of
/// `content`) plus the [`LogFile`]s the caller writes.
pub fn extract(
    msg: UserMessage,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (UserMessageLog, Vec<LogFile>) {
    let (content, files) = super::rich_content::extract_media(
        msg.content,
        &format!("{route_base}/messages"),
        id,
        message_index,
    );
    (
        UserMessageLog {
            content,
            name: msg.name,
        },
        files,
    )
}

//! Free-function port of `DeveloperMessage::extract`.

use objectiveai_sdk::agent::completions::message::{
    DeveloperMessage, DeveloperMessageLog,
};

use crate::filesystem::logs::LogFile;

/// Extract a `DeveloperMessage`'s content into per-leaf log files,
/// returning a [`DeveloperMessageLog`] (with `SimpleContentLog` in
/// place of `content`) plus the [`LogFile`]s the caller writes.
pub fn extract(
    msg: DeveloperMessage,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (DeveloperMessageLog, Vec<LogFile>) {
    let (content, files) = super::simple_content::extract_media(
        msg.content,
        &format!("{route_base}/messages"),
        id,
        message_index,
    );
    (
        DeveloperMessageLog {
            content,
            name: msg.name,
        },
        files,
    )
}

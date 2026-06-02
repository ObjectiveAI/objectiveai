//! Free-function port of `SystemMessage::extract`.

use objectiveai_sdk::agent::completions::message::{
    SystemMessage, SystemMessageLog,
};

use crate::filesystem::logs::LogFile;

/// Extract a `SystemMessage`'s content into per-leaf log files,
/// returning a [`SystemMessageLog`] (with `SimpleContentLog` in place
/// of `content`) plus the [`LogFile`]s the caller writes.
pub fn extract(
    msg: SystemMessage,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (SystemMessageLog, Vec<LogFile>) {
    let (content, files) = super::simple_content::extract_media(
        msg.content,
        &format!("{route_base}/messages"),
        id,
        message_index,
    );
    (
        SystemMessageLog {
            content,
            name: msg.name,
        },
        files,
    )
}

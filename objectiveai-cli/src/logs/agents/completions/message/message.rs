//! Free-function port of `Message::extract` (per-role dispatch).

use objectiveai_sdk::agent::completions::message::{Message, MessageLog};

use crate::filesystem::logs::LogFile;

/// Extract a `Message`'s content into per-leaf log files, returning a
/// [`MessageLog`] (with the per-role `*MessageLog` inside) plus the
/// [`LogFile`]s the caller writes. Dispatches per role to the
/// appropriate sibling extractor.
pub fn extract(
    msg: Message,
    route_base: &str,
    id: &str,
    message_index: u64,
) -> (MessageLog, Vec<LogFile>) {
    match msg {
        Message::Developer(m) => {
            let (log, files) = super::developer_message::extract(
                m,
                route_base,
                id,
                message_index,
            );
            (MessageLog::Developer(log), files)
        }
        Message::System(m) => {
            let (log, files) =
                super::system_message::extract(m, route_base, id, message_index);
            (MessageLog::System(log), files)
        }
        Message::User(m) => {
            let (log, files) =
                super::user_message::extract(m, route_base, id, message_index);
            (MessageLog::User(log), files)
        }
        Message::Assistant(m) => {
            let (log, files) = super::assistant_message::extract(
                m,
                route_base,
                id,
                message_index,
            );
            (MessageLog::Assistant(log), files)
        }
        Message::Tool(m) => {
            let (log, files) =
                super::tool_message::extract(m, route_base, id, message_index);
            (MessageLog::Tool(log), files)
        }
    }
}

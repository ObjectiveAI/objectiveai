//! Shared wire shapes and emit helpers used across CLI handlers.
//!
//! Every CLI handler emits via `objectiveai_cli_lib::output::Output<T>`,
//! where `T` is a small typed struct. This module collects the structs
//! and helpers that are shared across multiple commands; per-callsite
//! types stay inline at their callsite.

use serde::Serialize;

/// Shared shape for silent-success notifications (`config set`,
/// `favorites add/del/edit`, `instructions clear`, etc.). Emitted via
/// `Output::<Ok>::Notification(OK).emit()`. Wire shape:
/// `{"type":"notification","ok":true}`.
#[derive(Serialize)]
pub struct Ok {
    pub ok: bool,
}

pub const OK: Ok = Ok { ok: true };

/// Emit the contents of a single log read or subscribe. Mirrors the
/// upstream `LogContent` enum (which has no serde derives) into two
/// wire shapes discriminated by the field name (`content` for parsed
/// JSON, `content_data_url` for binary payloads).
pub fn emit_log_content(content: objectiveai::filesystem::logs::LogContent) {
    use objectiveai::filesystem::logs::LogContent;
    #[derive(Serialize)]
    struct JsonContent {
        content: serde_json::Value,
    }
    #[derive(Serialize)]
    struct DataUrlContent {
        content_data_url: String,
    }
    match content {
        LogContent::Json(v) => objectiveai_cli_lib::output::Output::<JsonContent>::Notification(
            JsonContent { content: v },
        )
        .emit(),
        LogContent::DataUrl(s) => {
            objectiveai_cli_lib::output::Output::<DataUrlContent>::Notification(DataUrlContent {
                content_data_url: s,
            })
            .emit()
        }
    }
}

pub fn emit_log_list(items: Vec<objectiveai::filesystem::logs::ListItem>) {
    #[derive(Serialize)]
    struct LogList {
        items: Vec<objectiveai::filesystem::logs::ListItem>,
    }
    objectiveai_cli_lib::output::Output::<LogList>::Notification(LogList { items }).emit();
}

pub fn emit_log_clear_count(count: u64) {
    #[derive(Serialize)]
    struct Cleared {
        cleared: u64,
    }
    objectiveai_cli_lib::output::Output::<Cleared>::Notification(Cleared { cleared: count })
        .emit();
}

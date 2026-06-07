//! Deferred-prompt storage for `agents message-queue {add, list, read id}`.
//! Bodies stubbed; SQL lands in stage 8.

use objectiveai_sdk::agent::completions::message::{
    File, ImageUrl, InputAudio, RichContent, RichContentPart, VideoUrl,
};
use objectiveai_sdk::cli::command::agents::message_queue::read::pending::ResponseItem;

use super::{Error, Pool};

/// One content row — typed payload of a single `prompt_contents.id`.
#[derive(Debug, Clone)]
pub enum ContentRow {
    Text(String),
    Image(ImageUrl),
    Audio(InputAudio),
    Video(VideoUrl),
    File(File),
}

/// Map one [`ContentRow`] to its matching SDK [`RichContentPart`] variant.
pub fn content_row_to_part(row: ContentRow) -> RichContentPart {
    match row {
        ContentRow::Text(text) => RichContentPart::Text { text },
        ContentRow::Image(image_url) => RichContentPart::ImageUrl { image_url },
        ContentRow::Audio(input_audio) => RichContentPart::InputAudio { input_audio },
        ContentRow::Video(video_url) => RichContentPart::VideoUrl { video_url },
        ContentRow::File(file) => RichContentPart::File { file },
    }
}

/// One drained prompt — carries enough metadata to re-INSERT the
/// original row.
#[derive(Debug, Clone)]
pub struct DrainedPrompt {
    pub agent_instance_hierarchy: Option<String>,
    pub agent_tag: Option<String>,
    pub key: Option<String>,
    pub enqueued_at: i64,
    pub content: RichContent,
}

/// One addressed delivery target.
#[derive(Debug, Clone)]
pub struct DeliveryTarget {
    pub agent_instance_hierarchy: String,
    pub agent_tag: Option<String>,
}

/// Atomic enqueue: inserts the `prompts` row + walks `content` into
/// per-kind tables. Returns the new `prompts.id`.
pub async fn enqueue_with_content(
    _pool: &Pool,
    _agent_instance_hierarchy: Option<String>,
    _agent_tag: Option<String>,
    _key: Option<String>,
    _content: RichContent,
) -> Result<i64, Error> {
    unimplemented!("db::prompts::enqueue_with_content — stage 8")
}

/// Look up a single content row by `prompt_contents.id`.
pub async fn read_content(
    _pool: &Pool,
    _id: i64,
) -> Result<Option<ContentRow>, Error> {
    unimplemented!("db::prompts::read_content — stage 8")
}

/// List all queued prompts visible under `parent`.
pub async fn list(_pool: &Pool, _parent: &str) -> Result<Vec<ResponseItem>, Error> {
    unimplemented!("db::prompts::list — stage 8")
}

/// Drain rows targeting `target_hierarchy`, `target_tag` (if some), or
/// any BOUND tag whose `agent_instance_hierarchy` equals
/// `target_hierarchy`. Deletes the matched rows in the same tx.
pub async fn drain_for_message(
    _pool: &Pool,
    _target_hierarchy: &str,
    _target_tag: Option<&str>,
) -> Result<Vec<DrainedPrompt>, Error> {
    unimplemented!("db::prompts::drain_for_message — stage 8")
}

/// Drain rows targeting `target_tag` (if some), or any PENDING tag
/// whose `(parent_agent_instance_hierarchy, agent_full_id)` pair
/// matches `(parent_hierarchy, agent_full_id)`.
pub async fn drain_for_spawn(
    _pool: &Pool,
    _parent_hierarchy: &str,
    _agent_full_id: &str,
    _target_tag: Option<&str>,
) -> Result<Vec<DrainedPrompt>, Error> {
    unimplemented!("db::prompts::drain_for_spawn — stage 8")
}

/// Re-INSERT every item in `items` as a fresh `prompts` row + content
/// rows. Empty `items` short-circuits to `Ok(())`.
pub async fn re_enqueue(_pool: &Pool, _items: Vec<DrainedPrompt>) -> Result<(), Error> {
    unimplemented!("db::prompts::re_enqueue — stage 8")
}

/// Atomically delete the `prompts` row with the given `id` and return
/// its reconstructed shape.
pub async fn delete_by_id(
    _pool: &Pool,
    _id: i64,
) -> Result<Option<DrainedPrompt>, Error> {
    unimplemented!("db::prompts::delete_by_id — stage 8")
}

/// Non-destructive read of every queue row in scope for an agent.
pub async fn read_for_message(
    _pool: &Pool,
    _target_hierarchy: &str,
    _parent_hierarchy: &str,
    _agent_full_id: &str,
) -> Result<Vec<(i64, RichContent)>, Error> {
    unimplemented!("db::prompts::read_for_message — stage 8")
}

/// Bulk-delete prompt rows by id, scoped to the same three-rule
/// predicate [`read_for_message`] uses.
pub async fn clear_by_ids(
    _pool: &Pool,
    _agent_instance_hierarchy: &str,
    _parent_hierarchy: &str,
    _agent_full_id: &str,
    _ids: Vec<i64>,
) -> Result<(), Error> {
    unimplemented!("db::prompts::clear_by_ids — stage 8")
}

/// Enumerate every distinct `(resolved hierarchy, agent_tag)` pair with
/// pending queue rows in the subtree rooted at `parent`.
pub async fn list_delivery_targets(
    _pool: &Pool,
    _parent: &str,
) -> Result<Vec<DeliveryTarget>, Error> {
    unimplemented!("db::prompts::list_delivery_targets — stage 8")
}

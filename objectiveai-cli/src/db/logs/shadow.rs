//! Shadow map keyed by `(table, response_id, index, sub_index)`.
//!
//! For every streaming-content row the writer might write, the
//! shadow remembers:
//!
//! - The row's body fingerprint (u64 hash of its column values).
//!
//! When the writer asks `shadow.record(value)`, the shadow returns one
//! of three [`WriteOp`]s:
//!
//! - `Insert`: this row id has never been seen → the writer issues a
//!   plain `INSERT`.
//! - `Update`: this row id was already written and the body changed →
//!   the writer issues a plain `UPDATE`.
//! - `Skip`: the row body is byte-identical to the last write → no DB
//!   call.
//!
//! Because the writer is the only caller and is single-instance per
//! stream, there is no concurrent contention on the keyspace — the
//! shadow's verdict is authoritative.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::row::{RowTable, RowValue};

/// What the writer should do with a particular row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOp {
    Insert,
    Update,
    Skip,
}

/// Identifies one row across every streaming-content table. The
/// (table, response_id, index, sub_index) tuple is the natural PK
/// shape for the streaming tables, generalized to a single hashable
/// type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowKey {
    pub table: RowTable,
    pub response_id: String,
    pub index: u64,
    /// `tool_call_index` for `AssistantResponseToolCalls`, `part_index`
    /// for content-part variants, `0` for the single-row-per-index
    /// slots (refusal / reasoning / tool_response).
    pub sub_index: u64,
}

impl RowKey {
    pub fn from_value(value: &RowValue<'_>) -> Self {
        let table = value.table();
        let (response_id, index, sub_index) = match value {
            RowValue::ToolResponse { response_id, index, .. }
            | RowValue::AssistantResponseRefusal { response_id, index, .. }
            | RowValue::AssistantResponseReasoning { response_id, index, .. } => {
                ((*response_id).to_owned(), *index, 0)
            }
            RowValue::AssistantResponseToolCalls {
                response_id, index, tool_call_index, ..
            } => ((*response_id).to_owned(), *index, *tool_call_index),
            RowValue::AssistantResponseContentText { response_id, index, part_index, .. }
            | RowValue::AssistantResponseContentImage { response_id, index, part_index, .. }
            | RowValue::AssistantResponseContentAudio { response_id, index, part_index, .. }
            | RowValue::AssistantResponseContentVideo { response_id, index, part_index, .. }
            | RowValue::AssistantResponseContentFile { response_id, index, part_index, .. }
            | RowValue::ToolResponseContentText { response_id, index, part_index, .. }
            | RowValue::ToolResponseContentImage { response_id, index, part_index, .. }
            | RowValue::ToolResponseContentAudio { response_id, index, part_index, .. }
            | RowValue::ToolResponseContentVideo { response_id, index, part_index, .. }
            | RowValue::ToolResponseContentFile { response_id, index, part_index, .. } => {
                ((*response_id).to_owned(), *index, *part_index)
            }
        };
        Self { table, response_id, index, sub_index }
    }
}

/// Identifies the tier blob (request OR response) for a particular
/// response id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlobKey {
    pub table: RowTable,
    pub response_id: String,
}

/// Hash the body columns of a [`RowValue`].
pub fn value_fingerprint(value: &RowValue<'_>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match value {
        RowValue::ToolResponse { tool_call_id, .. } => tool_call_id.hash(&mut hasher),
        RowValue::AssistantResponseRefusal { text, .. }
        | RowValue::AssistantResponseReasoning { text, .. }
        | RowValue::AssistantResponseContentText { text, .. }
        | RowValue::ToolResponseContentText { text, .. } => text.hash(&mut hasher),
        RowValue::AssistantResponseToolCalls { tool_call_id, arguments, .. } => {
            tool_call_id.hash(&mut hasher);
            arguments.hash(&mut hasher);
        }
        RowValue::AssistantResponseContentImage { image_url, .. }
        | RowValue::ToolResponseContentImage { image_url, .. } => image_url.hash(&mut hasher),
        RowValue::AssistantResponseContentAudio { input_audio, .. }
        | RowValue::ToolResponseContentAudio { input_audio, .. } => input_audio.hash(&mut hasher),
        RowValue::AssistantResponseContentVideo { video_url, is_input, .. }
        | RowValue::ToolResponseContentVideo { video_url, is_input, .. } => {
            video_url.hash(&mut hasher);
            is_input.hash(&mut hasher);
        }
        RowValue::AssistantResponseContentFile { file, .. }
        | RowValue::ToolResponseContentFile { file, .. } => file.hash(&mut hasher),
    }
    hasher.finish()
}

/// Compute a u64 fingerprint of a JSONB tier-blob body. The blob is
/// serialized to bytes once per tick and hashed for the diff check.
pub fn blob_fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[derive(Default)]
pub struct Shadow {
    rows: HashMap<RowKey, u64>,
    blobs: HashMap<BlobKey, u64>,
}

impl Shadow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up the row identified by `value`, compute its body
    /// fingerprint, and return the matching [`WriteOp`]. Stores the
    /// new fingerprint as the row's latest known body.
    pub fn record(&mut self, value: &RowValue<'_>) -> WriteOp {
        let key = RowKey::from_value(value);
        let fingerprint = value_fingerprint(value);
        match self.rows.get(&key) {
            Some(&existing) if existing == fingerprint => WriteOp::Skip,
            Some(_) => {
                self.rows.insert(key, fingerprint);
                WriteOp::Update
            }
            None => {
                self.rows.insert(key, fingerprint);
                WriteOp::Insert
            }
        }
    }

    /// Same idea as [`Self::record`] but for the tier blob — the
    /// caller passes the pre-serialized body bytes (so we can hash
    /// without re-encoding).
    pub fn record_blob(&mut self, table: RowTable, response_id: &str, body_bytes: &[u8]) -> WriteOp {
        let key = BlobKey { table, response_id: response_id.to_string() };
        let fingerprint = blob_fingerprint(body_bytes);
        match self.blobs.get(&key) {
            Some(&existing) if existing == fingerprint => WriteOp::Skip,
            Some(_) => {
                self.blobs.insert(key, fingerprint);
                WriteOp::Update
            }
            None => {
                self.blobs.insert(key, fingerprint);
                WriteOp::Insert
            }
        }
    }
}

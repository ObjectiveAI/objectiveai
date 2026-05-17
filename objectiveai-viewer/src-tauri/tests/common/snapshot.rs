//! Snapshot assertion + normalization helpers for the viewer
//! integration suite.
//!
//! Snapshots live under
//! `objectiveai-viewer/src-tauri/tests/snapshots/<name>` (any
//! extension — typically `.jsonl` for streaming flows, `.json` for
//! single-event flows). The test serializes its
//! collected-and-normalized output to a single `String`, then calls
//! [`assert_snapshot`]:
//!
//! - When `UPDATE_VIEWER_SNAPSHOTS=1` is set, the file is rewritten
//!   to match the new output (creating parent dirs if needed).
//! - Otherwise, the file's contents are read via `include_str!`
//!   (passed in by the caller, so the test binary embeds the
//!   expected text at compile time) and compared against the actual.
//!
//! Normalization is whatever the test author needs — the
//! [`normalize`] helper here scrubs the noisy non-deterministic
//! fields the viewer's bridge tends to surface (uuid-shaped `id`s,
//! `created` unix timestamps, mock call IDs, reasoning blobs, etc.).
//! Tests can call it as a post-pass before serializing.

use std::fs;
use std::path::Path;

const UPDATE_ENV_VAR: &str = "UPDATE_VIEWER_SNAPSHOTS";

/// Compare `actual` (already serialized + normalized) against
/// `expected` (the result of `include_str!` on the snapshot file).
///
/// On mismatch:
/// - If `UPDATE_VIEWER_SNAPSHOTS=1`, rewrite `path` with `actual` and
///   succeed.
/// - Otherwise panic with a clear-enough message; the test reporter
///   prints the diff (large snapshots are unavoidable here — better
///   to over-print than to hide what changed).
pub fn assert_snapshot(actual: &str, path: &str, expected: &str) {
    let update = std::env::var(UPDATE_ENV_VAR).as_deref() == Ok("1");
    if update {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).expect("create snapshot dir");
        }
        // Normalize line endings to \n so Windows test runs don't
        // produce CRLF-encoded snapshot files.
        let normalized = actual.replace("\r\n", "\n");
        fs::write(path, &normalized).expect("write snapshot");
        eprintln!("Updated snapshot: {path}");
        return;
    }
    let expected = expected.trim_end_matches('\n');
    let actual = actual.trim_end_matches('\n');
    if actual != expected {
        panic!(
            "snapshot mismatch at {path}\n\n--- expected ---\n{expected}\n\n--- actual ---\n{actual}\n\nRerun with UPDATE_VIEWER_SNAPSHOTS=1 to regenerate."
        );
    }
}

/// Walk a JSON value in place, replacing fields that are
/// guaranteed-non-deterministic (uuids, mock-call IDs, unix
/// timestamps, reasoning blobs, accumulated `upstream_id`s, etc.)
/// with stable placeholder strings/zeros so snapshots stay diffable
/// across reruns.
///
/// Conservative: only replaces fields whose names we know are
/// noisy. Adding a new field-name to scrub is one-line additions
/// to either `NOISY_STRING_KEYS` or `NOISY_NUMBER_KEYS`.
pub fn normalize(v: &mut serde_json::Value) {
    use serde_json::Value;

    /// Field names whose string values should be replaced with
    /// `"<scrubbed>"`.
    const NOISY_STRING_KEYS: &[&str] = &[
        "id",
        "upstream_id",
        "reasoning",
        "retry_token",
    ];
    /// Field names whose numeric values should be replaced with `0`.
    const NOISY_NUMBER_KEYS: &[&str] = &[
        "created",
    ];

    match v {
        Value::Object(map) => {
            for (k, child) in map.iter_mut() {
                if NOISY_STRING_KEYS.contains(&k.as_str()) && child.is_string() {
                    *child = Value::String("<scrubbed>".to_string());
                } else if NOISY_NUMBER_KEYS.contains(&k.as_str()) && child.is_number() {
                    *child = Value::Number(serde_json::Number::from(0));
                } else if k == "tool_calls" {
                    // Tool-call IDs are mock_<seed>-derived; scrub
                    // each call's `id` field.
                    if let Value::Array(arr) = child {
                        for call in arr.iter_mut() {
                            if let Value::Object(call_map) = call {
                                if let Some(id) = call_map.get_mut("id") {
                                    if id.is_string() {
                                        *id = Value::String("<scrubbed>".to_string());
                                    }
                                }
                                if let Some(func) = call_map.get_mut("function") {
                                    normalize(func);
                                }
                            }
                        }
                    }
                } else {
                    normalize(child);
                }
            }
        }
        Value::Array(arr) => {
            for child in arr.iter_mut() {
                normalize(child);
            }
        }
        _ => {}
    }
}

/// Serialize one `Event` to a single JSON line (no trailing
/// newline). Suitable for assembling JSONL snapshots.
pub fn event_to_jsonl_line(event: &objectiveai_sdk::viewer::Event) -> String {
    let mut value = serde_json::to_value(event).expect("serialize event");
    normalize(&mut value);
    serde_json::to_string(&value).expect("re-serialize event")
}

/// Concatenate the JSONL representations of a slice of events.
pub fn events_to_jsonl(events: &[objectiveai_sdk::viewer::Event]) -> String {
    let mut out = String::new();
    for event in events {
        out.push_str(&event_to_jsonl_line(event));
        out.push('\n');
    }
    out
}

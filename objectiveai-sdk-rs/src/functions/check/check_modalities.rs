//! Tracks which multimodal content types (image, audio, video, file) appear in
//! input schemas and compiled vector completion tasks.
//!
//! Used by both leaf scalar and leaf vector checkers to ensure that any multimodal
//! type declared in the input schema actually appears in at least one compiled
//! task's messages or responses.

use crate::agent::completions::message::{Message, RichContent, RichContentPart};
use crate::functions::expression::InputSchema;
use crate::functions::VectorCompletionTask;

/// Indices: 0 = image, 1 = audio, 2 = video, 3 = file.
pub type ModalityFlags = [bool; 4];

const IMAGE: usize = 0;
const AUDIO: usize = 1;
const VIDEO: usize = 2;
const FILE: usize = 3;

/// Walks an `InputSchema` recursively and sets flags for any multimodal types found.
pub fn collect_schema_modalities(schema: &InputSchema, flags: &mut ModalityFlags) {
    match schema {
        InputSchema::Image(_) => flags[IMAGE] = true,
        InputSchema::Audio(_) => flags[AUDIO] = true,
        InputSchema::Video(_) => flags[VIDEO] = true,
        InputSchema::File(_) => flags[FILE] = true,
        InputSchema::Object(obj) => {
            for prop_schema in obj.properties.values() {
                collect_schema_modalities(prop_schema, flags);
            }
        }
        InputSchema::Array(arr) => {
            collect_schema_modalities(&arr.items, flags);
        }
        InputSchema::AnyOf(any_of) => {
            for option in &any_of.any_of {
                collect_schema_modalities(option, flags);
            }
        }
        InputSchema::String(_)
        | InputSchema::Integer(_)
        | InputSchema::Number(_)
        | InputSchema::Boolean(_) => {}
    }
}

/// Scans a compiled `VectorCompletionTask`'s messages and responses for
/// multimodal content parts and sets the corresponding flags.
pub fn collect_task_modalities(
    task: &VectorCompletionTask,
    flags: &mut ModalityFlags,
) {
    for msg in &task.messages {
        if let Some(content) = message_rich_content(msg) {
            collect_rich_content_modalities(content, flags);
        }
    }
    for response in &task.responses {
        collect_rich_content_modalities(response, flags);
    }
}

/// Checks that every modality in `schema_flags` is also present in `task_flags`.
/// Returns an error string listing missing modalities, or Ok(()).
pub fn check_modality_coverage(
    schema_flags: &ModalityFlags,
    task_flags: &ModalityFlags,
    code: &str,
) -> Result<(), String> {
    const NAMES: [&str; 4] = ["image", "audio", "video", "file"];
    let mut missing = Vec::new();
    for i in 0..4 {
        if schema_flags[i] && !task_flags[i] {
            missing.push(NAMES[i]);
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{}: input_schema declares multimodal type(s) [{}] but no compiled task \
             contains content parts of that type — tasks must use multimodal input, \
             not convert it to text with str()",
            code,
            missing.join(", "),
        ))
    }
}

/// Extracts the `RichContent` from a compiled `Message`, if it has rich content.
fn message_rich_content(msg: &Message) -> Option<&RichContent> {
    match msg {
        Message::User(u) => Some(&u.content),
        Message::Tool(t) => Some(&t.content),
        Message::Assistant(a) => a.content.as_ref(),
        // Developer and System use SimpleContent, which cannot contain multimodal.
        Message::Developer(_) | Message::System(_) => None,
    }
}

/// Scans `RichContent` for multimodal content parts.
fn collect_rich_content_modalities(
    content: &RichContent,
    flags: &mut ModalityFlags,
) {
    if let RichContent::Parts(parts) = content {
        for part in parts {
            match part {
                RichContentPart::ImageUrl { .. } => flags[IMAGE] = true,
                RichContentPart::InputAudio { .. } => flags[AUDIO] = true,
                RichContentPart::InputVideo { .. } | RichContentPart::VideoUrl { .. } => {
                    flags[VIDEO] = true
                }
                RichContentPart::File { .. } => flags[FILE] = true,
                RichContentPart::Text { .. } => {}
            }
        }
    }
}

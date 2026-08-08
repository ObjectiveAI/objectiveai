//! Streaming tool-call deltas.

use serde::{Deserialize, Serialize};

/// One tool call, streamed in pieces.
///
/// Deltas are matched by `index`, not by `id` — the id arrives with the
/// first delta and later ones carry only argument text, so `index` is
/// the only field present on every piece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AssistantToolCallDelta {
    /// Position of this tool call within the turn. The join key.
    pub index: u64,
    /// Always `function`. Only present on the first delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssistantToolCallType>,
    /// The call's unique id. Only present on the first delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The call itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<AssistantToolCallFunctionDelta>,
}

impl AssistantToolCallDelta {
    /// Accumulate another delta for the same `index` into this one.
    pub fn push(&mut self, other: &AssistantToolCallDelta) {
        if self.r#type.is_none() {
            self.r#type = other.r#type;
        }
        if self.id.is_none() {
            self.id = other.id.clone();
        }
        match (&mut self.function, &other.function) {
            (Some(self_function), Some(other_function)) => {
                self_function.push(other_function);
            }
            (None, Some(other_function)) => {
                self.function = Some(other_function.clone());
            }
            _ => {}
        }
    }
}

/// The kind of tool call — always a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantToolCallType {
    #[serde(rename = "function")]
    #[default]
    Function,
}

/// The function half of a tool call, streamed in pieces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AssistantToolCallFunctionDelta {
    /// The function name. Arrives once, on the first delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// JSON-encoded arguments, CONCATENATED across deltas. A single
    /// delta is not valid JSON on its own; only the accumulation is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl AssistantToolCallFunctionDelta {
    /// Accumulate another delta into this one: the name is taken once,
    /// the arguments are appended.
    pub fn push(&mut self, other: &AssistantToolCallFunctionDelta) {
        if self.name.is_none() {
            self.name = other.name.clone();
        }
        super::util::push_option_string(&mut self.arguments, &other.arguments);
    }
}

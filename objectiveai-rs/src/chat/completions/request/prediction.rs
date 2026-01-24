//! Predicted output for speculative decoding.

use serde::{Deserialize, Serialize};

/// A predicted output for speculative decoding.
///
/// When you have a good guess about what the model will output, providing a
/// prediction can significantly speed up generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Prediction {
    /// A content prediction.
    Content { content: PredictionContent },
}

/// The content of a prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PredictionContent {
    /// Plain text prediction.
    Text(String),
    /// Multi-part prediction.
    Parts(Vec<PredictionContentPart>),
}

/// A part of a prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PredictionContentPart {
    /// A text part.
    Text { text: String },
}

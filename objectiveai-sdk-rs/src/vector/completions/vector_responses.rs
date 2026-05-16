//! Vector responses type for vector completions.
//!
//! A vector response is a single option that LLMs can vote for in a vector
//! completion. Each response can be plain text or multimodal content (images,
//! audio, video, files).

use crate::agent::completions::message::RichContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The list of response options in a vector completion request.
///
/// Each element is a [`RichContent`] value that an LLM can vote for.
/// Responses can be plain text strings or multi-part content containing
/// text, images, audio, video, or files.
///
/// # Minimum Length
///
/// A vector completion requires at least 2 responses to vote between.
///
/// # Examples
///
/// Plain text responses:
/// ```json
/// ["Yes", "No", "Maybe"]
/// ```
///
/// Multimodal responses:
/// ```json
/// [
///   [{"type": "text", "text": "Option A"}, {"type": "image_url", "image_url": {"url": "https://example.com/a.png"}}],
///   [{"type": "text", "text": "Option B"}, {"type": "image_url", "image_url": {"url": "https://example.com/b.png"}}]
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.VectorResponses")]
#[serde(transparent)]
pub struct VectorResponses(pub Vec<RichContent>);

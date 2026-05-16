//! Vector response transformation for LLM prompts.
//!
//! Converts vector response options into prompt content parts with labeled keys.

/// Transforms vector responses into prompt content parts.
///
/// Formats responses as a JSON-like structure with prefix keys (e.g., `` `A` ``)
/// as labels, suitable for inclusion in the user message prompt.
pub fn into_parts_for_prompt(
    vector_responses: &[objectiveai_sdk::agent::completions::message::RichContent],
    vector_pfx_indices: &[(String, usize)],
) -> Vec<objectiveai_sdk::agent::completions::message::RichContentPart> {
    let mut parts = Vec::new();
    for (i, (vector_pfx_key, vector_response_index)) in vector_pfx_indices.iter().enumerate() {
        if i == 0 {
            parts.push(
                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                    text: format!("{{\n    \"{}\": \"", vector_pfx_key),
                },
            );
        } else {
            parts.push(
                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                    text: format!("\",\n    \"{}\": \"", vector_pfx_key),
                },
            );
        }
        match &vector_responses[*vector_response_index] {
            objectiveai_sdk::agent::completions::message::RichContent::Text(text) => {
                parts.push(
                    objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                        text: json_escape::escape_str(text).to_string(),
                    },
                );
            }
            objectiveai_sdk::agent::completions::message::RichContent::Parts(rich_parts) => {
                for rich_part in rich_parts {
                    match rich_part {
                        objectiveai_sdk::agent::completions::message::RichContentPart::Text { text } => {
                            parts.push(
                                objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                                    text: json_escape::escape_str(text).to_string(),
                                },
                            );
                        }
                        part => {
                            parts.push(part.clone());
                        }
                    }
                }
            }
        }
    }
    if parts.len() > 0 {
        parts.push(
            objectiveai_sdk::agent::completions::message::RichContentPart::Text {
                text: "\"\n}".to_string(),
            },
        );
    }
    parts
}

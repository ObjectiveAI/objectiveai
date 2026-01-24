//! Prompt construction for OpenRouter requests.
//!
//! Handles merging prefix/suffix messages from Ensemble LLM configurations
//! with the request messages, and constructs vector voting prompts.

use crate::vector;

/// Constructs the message array for a chat completion.
///
/// Concatenates the Ensemble LLM's prefix messages, the request messages,
/// and the Ensemble LLM's suffix messages.
pub fn new_for_chat(
    ensemble_llm_prefix: Option<&[objectiveai::chat::completions::request::Message]>,
    request: &[objectiveai::chat::completions::request::Message],
    ensemble_llm_suffix: Option<&[objectiveai::chat::completions::request::Message]>,
) -> Vec<objectiveai::chat::completions::request::Message> {
    match (ensemble_llm_prefix, ensemble_llm_suffix) {
        (Some(ensemble_llm_prefix), Some(ensemble_llm_suffix)) => {
            let mut messages = Vec::with_capacity(
                ensemble_llm_prefix.len() + request.len() + ensemble_llm_suffix.len(),
            );
            messages.extend_from_slice(ensemble_llm_prefix);
            messages.extend_from_slice(request);
            messages.extend_from_slice(ensemble_llm_suffix);
            messages
        }
        (Some(ensemble_llm_prefix), None) => {
            let mut messages = Vec::with_capacity(ensemble_llm_prefix.len() + request.len());
            messages.extend_from_slice(ensemble_llm_prefix);
            messages.extend_from_slice(request);
            messages
        }
        (None, Some(ensemble_llm_suffix)) => {
            let mut messages = Vec::with_capacity(request.len() + ensemble_llm_suffix.len());
            messages.extend_from_slice(request);
            messages.extend_from_slice(ensemble_llm_suffix);
            messages
        }
        (None, None) => request.to_vec(),
    }
}

/// Constructs the message array for a vector completion vote.
///
/// Appends the labeled response options to the last user message and adds
/// voting instructions to the system message based on the output mode.
pub fn new_for_vector(
    vector_responses: &[objectiveai::chat::completions::request::RichContent],
    vector_pfx_indices: &[(String, usize)],
    ensemble_llm_output_mode: objectiveai::ensemble_llm::OutputMode,
    ensemble_llm_prefix: Option<&[objectiveai::chat::completions::request::Message]>,
    request: &[objectiveai::chat::completions::request::Message],
    ensemble_llm_suffix: Option<&[objectiveai::chat::completions::request::Message]>,
) -> Vec<objectiveai::chat::completions::request::Message> {
    // convert vector responses into rich content parts for prompt
    let vector_responses_for_prompt = vector::completions::vector_responses::into_parts_for_prompt(
        vector_responses,
        vector_pfx_indices,
    );

    // merge messages
    let mut messages = new_for_chat(ensemble_llm_prefix, request, ensemble_llm_suffix);

    // handle user message transform
    // append vector responses to last user message, or create one if none
    let mut user_append_content = None;
    let mut user_message_i = messages.len();
    for (i, message) in messages.iter_mut().enumerate().rev() {
        if let objectiveai::chat::completions::request::Message::User(user_message) = message {
            user_append_content = Some(&mut user_message.content);
            user_message_i = i;
            break;
        }
    }
    if user_append_content.is_none() {
        messages.push(objectiveai::chat::completions::request::Message::User(
            objectiveai::chat::completions::request::UserMessage {
                content: objectiveai::chat::completions::request::RichContent::Parts(
                    Vec::with_capacity(1 + vector_responses_for_prompt.len()),
                ),
                name: None,
            },
        ));
        user_append_content = match messages.last_mut().unwrap() {
            objectiveai::chat::completions::request::Message::User(user_message) => {
                Some(&mut user_message.content)
            }
            _ => unreachable!(),
        };
    }
    let user_append_content = user_append_content.unwrap();
    let user_append_content_parts = match user_append_content {
        objectiveai::chat::completions::request::RichContent::Text(text) => {
            let mut parts = Vec::with_capacity(2 + vector_responses_for_prompt.len());
            parts.push(
                objectiveai::chat::completions::request::RichContentPart::Text {
                    text: text.clone(),
                },
            );
            *user_append_content =
                objectiveai::chat::completions::request::RichContent::Parts(parts);
            match user_append_content {
                objectiveai::chat::completions::request::RichContent::Parts(parts) => parts,
                _ => unreachable!(),
            }
        }
        objectiveai::chat::completions::request::RichContent::Parts(parts) => parts,
    };
    user_append_content_parts.push(
        objectiveai::chat::completions::request::RichContentPart::Text {
            text: if user_append_content_parts.is_empty() {
                "Select the response:\n\n".to_string()
            } else {
                "\n\nSelect the response:\n\n".to_string()
            },
        },
    );
    user_append_content_parts.extend(vector_responses_for_prompt);

    // handle system message transform
    // append instruction to last system message, or create one if none
    if let objectiveai::ensemble_llm::OutputMode::Instruction = ensemble_llm_output_mode {
        let mut system_append_content = None;
        for (i, message) in messages.iter_mut().enumerate().rev() {
            if i <= user_message_i {
                break;
            } else if let objectiveai::chat::completions::request::Message::System(system_message) =
                message
            {
                system_append_content = Some(&mut system_message.content);
                break;
            }
        }
        if system_append_content.is_none() {
            messages.push(objectiveai::chat::completions::request::Message::System(
                objectiveai::chat::completions::request::SystemMessage {
                    content: objectiveai::chat::completions::request::SimpleContent::Parts(
                        Vec::with_capacity(1),
                    ),
                    name: None,
                },
            ));
            system_append_content = match messages.last_mut().unwrap() {
                objectiveai::chat::completions::request::Message::System(system_message) => {
                    Some(&mut system_message.content)
                }
                _ => unreachable!(),
            };
        }
        let system_append_content = system_append_content.unwrap();
        let system_append_content_parts = match system_append_content {
            objectiveai::chat::completions::request::SimpleContent::Text(text) => {
                let mut parts = Vec::with_capacity(2);
                parts.push(
                    objectiveai::chat::completions::request::SimpleContentPart::Text {
                        text: text.clone(),
                    },
                );
                *system_append_content =
                    objectiveai::chat::completions::request::SimpleContent::Parts(parts);
                match system_append_content {
                    objectiveai::chat::completions::request::SimpleContent::Parts(parts) => parts,
                    _ => unreachable!(),
                }
            }
            objectiveai::chat::completions::request::SimpleContent::Parts(parts) => parts,
        };
        // TODO: optimize this, it allocates a vec and clones strings
        system_append_content_parts.push(
            objectiveai::chat::completions::request::SimpleContentPart::Text {
                text: if system_append_content_parts.is_empty() {
                    format!(
                        "Output one response key including backticks\n- {}",
                        vector_pfx_indices
                            .iter()
                            .map(|(key, _)| key.clone())
                            .collect::<Vec<_>>()
                            .join("\n- ")
                    )
                } else {
                    format!(
                        "\n\nOutput one response key including backticks:\n- {}",
                        vector_pfx_indices
                            .iter()
                            .map(|(key, _)| key.clone())
                            .collect::<Vec<_>>()
                            .join("\n- ")
                    )
                },
            },
        );
    }

    // return transformed messages
    messages
}

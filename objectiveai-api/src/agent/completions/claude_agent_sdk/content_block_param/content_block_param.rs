use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ContentBlockParam {
    Text(super::TextBlockParam),
    Image(super::ImageBlockParam),
    Document(super::DocumentBlockParam),
    SearchResult(super::SearchResultBlockParam),
    Thinking(super::ThinkingBlockParam),
    RedactedThinking(super::RedactedThinkingBlockParam),
    ToolUse(super::ToolUseBlockParam),
    ToolResult(super::ToolResultBlockParam),
    ServerToolUse(super::ServerToolUseBlockParam),
    WebSearchToolResult(super::WebSearchToolResultBlockParam),
    WebFetchToolResult(super::WebFetchToolResultBlockParam),
    CodeExecutionToolResult(super::CodeExecutionToolResultBlockParam),
    BashCodeExecutionToolResult(super::BashCodeExecutionToolResultBlockParam),
    TextEditorCodeExecutionToolResult(super::TextEditorCodeExecutionToolResultBlockParam),
    ToolSearchToolResult(super::ToolSearchToolResultBlockParam),
    ContainerUpload(super::ContainerUploadBlockParam),
}

impl TryFrom<&objectiveai_sdk::agent::completions::message::RichContentPart> for ContentBlockParam {
    type Error = String;

    fn try_from(
        part: &objectiveai_sdk::agent::completions::message::RichContentPart,
    ) -> Result<Self, Self::Error> {
        use objectiveai_sdk::agent::completions::message::RichContentPart;

        match part {
            RichContentPart::Text { text } => Ok(ContentBlockParam::Text(super::TextBlockParam {
                text: text.clone(),
                r#type: super::TextBlockParamType::Text,
                cache_control: None,
                citations: None,
            })),

            RichContentPart::ImageUrl { image_url } => {
                super::ImageBlockParam::try_from(image_url).map(ContentBlockParam::Image)
            }

            RichContentPart::InputAudio { input_audio } => Err(format!(
                "unsupported content type: audio ({} format, {} base64 chars)",
                input_audio.format,
                input_audio.data.len()
            )),

            RichContentPart::VideoUrl { .. }
            | RichContentPart::InputVideo { .. } => {
                Err("unsupported content type: video".to_string())
            }

            RichContentPart::File { file } => {
                super::DocumentBlockParam::try_from(file).map(ContentBlockParam::Document)
            }
        }
    }
}

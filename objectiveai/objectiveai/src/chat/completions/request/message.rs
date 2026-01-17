//! Message types for chat completions.
//!
//! Messages represent the conversation history sent to the model. Each message
//! has a role (system, user, assistant, tool, or developer) and content.

use crate::functions;
use serde::{Deserialize, Serialize};

/// Utilities for working with message prompts.
pub mod prompt {
    /// Prepares a list of messages by normalizing each one.
    pub fn prepare(messages: &mut Vec<super::Message>) {
        messages.iter_mut().for_each(super::Message::prepare);
    }

    /// Computes a content-addressed ID for a list of messages.
    pub fn id(messages: &[super::Message]) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(messages).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    /// A developer message (similar to system, but from the developer).
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    /// A system message setting context or instructions.
    #[serde(rename = "system")]
    System(SystemMessage),
    /// A user message from the end user.
    #[serde(rename = "user")]
    User(UserMessage),
    /// An assistant message (model's previous response).
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// A tool message containing the result of a tool call.
    #[serde(rename = "tool")]
    Tool(ToolMessage),
}

impl Message {
    pub fn prepare(&mut self) {
        match self {
            Message::Developer(msg) => msg.prepare(),
            Message::System(msg) => msg.prepare(),
            Message::User(msg) => msg.prepare(),
            Message::Assistant(msg) => msg.prepare(),
            Message::Tool(msg) => msg.prepare(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum MessageExpression {
    #[serde(rename = "developer")]
    Developer(DeveloperMessageExpression),
    #[serde(rename = "system")]
    System(SystemMessageExpression),
    #[serde(rename = "user")]
    User(UserMessageExpression),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessageExpression),
    #[serde(rename = "tool")]
    Tool(ToolMessageExpression),
}

impl MessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<Message, functions::expression::ExpressionError> {
        match self {
            MessageExpression::Developer(msg) => {
                msg.compile(params).map(Message::Developer)
            }
            MessageExpression::System(msg) => {
                msg.compile(params).map(Message::System)
            }
            MessageExpression::User(msg) => {
                msg.compile(params).map(Message::User)
            }
            MessageExpression::Assistant(msg) => {
                msg.compile(params).map(Message::Assistant)
            }
            MessageExpression::Tool(msg) => {
                msg.compile(params).map(Message::Tool)
            }
        }
    }
}

/// A developer message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperMessage {
    /// The message content.
    pub content: SimpleContent,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DeveloperMessage {
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperMessageExpression {
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl DeveloperMessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<DeveloperMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self
            .name
            .map(|name| name.compile_one(params))
            .transpose()?
            .flatten();
        Ok(DeveloperMessage { content, name })
    }
}

/// A system message setting context or instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    /// The message content.
    pub content: SimpleContent,
    /// Optional name for the message author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl SystemMessage {
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessageExpression {
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl SystemMessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SystemMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self
            .name
            .map(|name| name.compile_one(params))
            .transpose()?
            .flatten();
        Ok(SystemMessage { content, name })
    }
}

/// A user message from the end user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    /// The message content (supports text, images, audio, video, files).
    pub content: RichContent,
    /// Optional name for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl UserMessage {
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageExpression {
    pub content: functions::expression::WithExpression<RichContentExpression>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl UserMessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<UserMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let name = self
            .name
            .map(|name| name.compile_one(params))
            .transpose()?
            .flatten();
        Ok(UserMessage { content, name })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMessage {
    pub content: RichContent,
    pub tool_call_id: String,
}

impl ToolMessage {
    pub fn prepare(&mut self) {
        self.content.prepare();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMessageExpression {
    pub content: functions::expression::WithExpression<RichContentExpression>,
    pub tool_call_id: functions::expression::WithExpression<String>,
}

impl ToolMessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<ToolMessage, functions::expression::ExpressionError> {
        let content = self.content.compile_one(params)?.compile(params)?;
        let tool_call_id = self.tool_call_id.compile_one(params)?;
        Ok(ToolMessage {
            content,
            tool_call_id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RichContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

impl AssistantMessage {
    pub fn prepare(&mut self) {
        if let Some(content) = &mut self.content {
            content.prepare();
            if content.is_empty() {
                self.content = None;
            }
        }
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
        if self.refusal.as_ref().is_some_and(String::is_empty) {
            self.refusal = None;
        }
        if let Some(tool_calls) = &mut self.tool_calls {
            tool_calls.retain(|tool_call| !tool_call.is_empty());
            if tool_calls.is_empty() {
                self.tool_calls = None;
            }
        }
        if self.reasoning.as_ref().is_some_and(String::is_empty) {
            self.reasoning = None;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageExpression {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<
        functions::expression::WithExpression<Option<RichContentExpression>>,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<functions::expression::WithExpression<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<
        functions::expression::WithExpression<
            Option<
                Vec<
                    functions::expression::WithExpression<
                        AssistantToolCallExpression,
                    >,
                >,
            >,
        >,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning:
        Option<functions::expression::WithExpression<Option<String>>>,
}

impl AssistantMessageExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantMessage, functions::expression::ExpressionError> {
        let content = self
            .content
            .map(|content| content.compile_one(params))
            .transpose()?
            .flatten()
            .map(|content| content.compile(params))
            .transpose()?;
        let name = self
            .name
            .map(|name| name.compile_one(params))
            .transpose()?
            .flatten();
        let refusal = self
            .refusal
            .map(|refusal| refusal.compile_one(params))
            .transpose()?
            .flatten();
        let tool_calls = self
            .tool_calls
            .map(|tool_calls| {
                let tool_calls = tool_calls.compile_one(params)?;
                Ok::<_, functions::expression::ExpressionError>(tool_calls.map(
                    |tool_calls| {
                        let mut compiled_tool_calls =
                            Vec::with_capacity(tool_calls.len());
                        for tool_call in tool_calls {
                            match tool_call.compile_one_or_many(params)? {
                                functions::expression::OneOrMany::One(
                                    one_tool_call,
                                ) => {
                                    compiled_tool_calls
                                        .push(one_tool_call.compile(params)?);
                                }
                                functions::expression::OneOrMany::Many(
                                    many_tool_calls,
                                ) => {
                                    for tool_call in many_tool_calls {
                                        compiled_tool_calls
                                            .push(tool_call.compile(params)?);
                                    }
                                }
                            }
                        }
                        Ok::<_, functions::expression::ExpressionError>(
                            compiled_tool_calls,
                        )
                    },
                ))
            })
            .transpose()?
            .flatten()
            .transpose()?;
        let reasoning = self
            .reasoning
            .map(|reasoning| reasoning.compile_one(params))
            .transpose()?
            .flatten();
        Ok(AssistantMessage {
            content,
            name,
            refusal,
            tool_calls,
            reasoning,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCall {
    Function {
        id: String,
        function: AssistantToolCallFunction,
    },
}

impl AssistantToolCall {
    pub fn is_empty(&self) -> bool {
        match self {
            AssistantToolCall::Function { id, function } => {
                id.is_empty() && function.is_empty()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCallExpression {
    Function {
        id: functions::expression::WithExpression<String>,
        function: functions::expression::WithExpression<
            AssistantToolCallFunctionExpression,
        >,
    },
}

impl AssistantToolCallExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantToolCall, functions::expression::ExpressionError> {
        match self {
            AssistantToolCallExpression::Function { id, function } => {
                let id = id.compile_one(params)?;
                let function = function.compile_one(params)?.compile(params)?;
                Ok(AssistantToolCall::Function { id, function })
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantToolCallFunction {
    pub name: String,
    pub arguments: String,
}

impl AssistantToolCallFunction {
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.arguments.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantToolCallFunctionExpression {
    pub name: functions::expression::WithExpression<String>,
    pub arguments: functions::expression::WithExpression<String>,
}

impl AssistantToolCallFunctionExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<AssistantToolCallFunction, functions::expression::ExpressionError>
    {
        let name = self.name.compile_one(params)?;
        let arguments = self.arguments.compile_one(params)?;
        Ok(AssistantToolCallFunction { name, arguments })
    }
}

/// Simple text content for system/developer messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SimpleContent {
    /// Plain text content.
    Text(String),
    /// Multi-part text content.
    Parts(Vec<SimpleContentPart>),
}

impl SimpleContent {
    pub fn prepare(&mut self) {
        match self {
            SimpleContent::Text(_) => {}
            SimpleContent::Parts(parts) => {
                let text_len = parts
                    .iter()
                    .map(|part| match part {
                        SimpleContentPart::Text { text } => text.len(),
                    })
                    .sum();
                let mut text = String::with_capacity(text_len);
                for part in parts {
                    match part {
                        SimpleContentPart::Text { text: part_text } => {
                            text.push_str(&part_text);
                        }
                    }
                }
                *self = SimpleContent::Text(text)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SimpleContentExpression {
    Text(String),
    Parts(Vec<functions::expression::WithExpression<SimpleContentPart>>),
}

impl SimpleContentExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SimpleContent, functions::expression::ExpressionError> {
        match self {
            SimpleContentExpression::Text(text) => {
                Ok(SimpleContent::Text(text))
            }
            SimpleContentExpression::Parts(parts) => {
                let mut compiled_parts = Vec::with_capacity(parts.len());
                for part in parts {
                    match part.compile_one_or_many(params)? {
                        functions::expression::OneOrMany::One(one_part) => {
                            compiled_parts.push(one_part);
                        }
                        functions::expression::OneOrMany::Many(many_parts) => {
                            compiled_parts.extend(many_parts);
                        }
                    }
                }
                Ok(SimpleContent::Parts(compiled_parts))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimpleContentPart {
    Text { text: String },
}

/// Rich content for user/assistant messages (supports multimodal input).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RichContent {
    /// Plain text content.
    Text(String),
    /// Multi-part content (text, images, audio, video, files).
    Parts(Vec<RichContentPart>),
}

impl RichContent {
    pub fn prepare(&mut self) {
        // nothing to prepare for plain text
        let parts = match self {
            RichContent::Text(_) => return,
            RichContent::Parts(parts) => parts,
        };

        // prepare all parts
        parts.iter_mut().for_each(RichContentPart::prepare);

        // join consecutive text parts + remove empty parts
        let mut final_parts = Vec::with_capacity(parts.len());
        let mut buffer: Option<String> = None;
        for part in parts.drain(..) {
            match part {
                part if part.is_empty() => continue,
                RichContentPart::Text { text } => {
                    if let Some(buffer) = &mut buffer {
                        buffer.push_str(&text);
                    } else {
                        buffer = Some(text);
                    }
                }
                part => {
                    if let Some(buffer) = buffer.take() {
                        final_parts
                            .push(RichContentPart::Text { text: buffer });
                    }
                    final_parts.push(part);
                }
            }
        }
        if let Some(buffer) = buffer.take() {
            final_parts.push(RichContentPart::Text { text: buffer });
        }

        // replace self with final parts
        if final_parts.len() == 1
            && matches!(&final_parts[0], RichContentPart::Text { .. })
        {
            match final_parts.into_iter().next() {
                Some(RichContentPart::Text { text }) => {
                    *self = RichContent::Text(text);
                }
                _ => unreachable!(),
            }
        } else {
            *self = RichContent::Parts(final_parts);
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RichContent::Text(text) => text.is_empty(),
            RichContent::Parts(parts) => parts.is_empty(),
        }
    }

    pub fn id(&self) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RichContentExpression {
    Text(String),
    Parts(Vec<functions::expression::WithExpression<RichContentPart>>),
}

impl RichContentExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<RichContent, functions::expression::ExpressionError> {
        match self {
            RichContentExpression::Text(text) => Ok(RichContent::Text(text)),
            RichContentExpression::Parts(parts) => {
                let mut compiled_parts = Vec::with_capacity(parts.len());
                for part in parts {
                    match part.compile_one_or_many(params)? {
                        functions::expression::OneOrMany::One(one_part) => {
                            compiled_parts.push(one_part);
                        }
                        functions::expression::OneOrMany::Many(many_parts) => {
                            compiled_parts.extend(many_parts);
                        }
                    }
                }
                Ok(RichContent::Parts(compiled_parts))
            }
        }
    }
}

/// A part of rich content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichContentPart {
    /// Text content.
    Text { text: String },
    /// An image URL.
    ImageUrl { image_url: ImageUrl },
    /// Audio input.
    InputAudio { input_audio: InputAudio },
    /// Video input.
    InputVideo { video_url: VideoUrl },
    /// A video URL.
    VideoUrl { video_url: VideoUrl },
    /// A file.
    File { file: File },
}

impl RichContentPart {
    pub fn prepare(&mut self) {
        match self {
            RichContentPart::Text { .. } => {}
            RichContentPart::ImageUrl { image_url } => {
                image_url.prepare();
            }
            RichContentPart::InputAudio { .. } => {}
            RichContentPart::InputVideo { .. } => {}
            RichContentPart::VideoUrl { .. } => {}
            RichContentPart::File { file } => {
                file.prepare();
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RichContentPart::Text { text } => text.is_empty(),
            RichContentPart::ImageUrl { image_url } => image_url.is_empty(),
            RichContentPart::InputAudio { input_audio } => {
                input_audio.is_empty()
            }
            RichContentPart::InputVideo { video_url } => video_url.is_empty(),
            RichContentPart::VideoUrl { video_url } => video_url.is_empty(),
            RichContentPart::File { file } => file.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageUrlDetail>,
}

impl ImageUrl {
    pub fn prepare(&mut self) {
        if matches!(self.detail, Some(ImageUrlDetail::Auto)) {
            self.detail = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.url.is_empty() && self.detail.is_none()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageUrlDetail {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "high")]
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudio {
    pub data: String,
    pub format: String,
}

impl InputAudio {
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.format.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUrl {
    pub url: String,
}

impl VideoUrl {
    pub fn is_empty(&self) -> bool {
        self.url.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
}

impl File {
    pub fn prepare(&mut self) {
        if self.file_data.as_ref().is_some_and(String::is_empty) {
            self.file_data = None;
        }
        if self.file_id.as_ref().is_some_and(String::is_empty) {
            self.file_id = None;
        }
        if self.filename.as_ref().is_some_and(String::is_empty) {
            self.filename = None;
        }
        if self.file_url.as_ref().is_some_and(String::is_empty) {
            self.file_url = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.file_data.is_none()
            && self.file_id.is_none()
            && self.filename.is_none()
            && self.file_url.is_none()
    }
}

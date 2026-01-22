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
    /// Prepares the message for sending by normalizing its content.
    ///
    /// This method consolidates consecutive text parts, removes empty parts,
    /// and normalizes optional fields (setting empty strings to `None`).
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

/// A message with JMESPath expressions for dynamic content.
///
/// This is the expression variant of [`Message`] used in function definitions
/// where message content can be computed from the function input at runtime.
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
    /// Compiles the expression into a concrete [`Message`].
    ///
    /// Evaluates all JMESPath expressions using the provided parameters
    /// and returns the resulting message.
    ///
    /// # Errors
    ///
    /// Returns an error if any expression evaluation fails.
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
    /// Prepares the message by normalizing content and optional fields.
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

/// Expression variant of [`DeveloperMessage`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    /// Optional name expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl DeveloperMessageExpression {
    /// Compiles the expression into a concrete [`DeveloperMessage`].
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
    /// Prepares the message by normalizing content and optional fields.
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

/// Expression variant of [`SystemMessage`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<SimpleContentExpression>,
    /// Optional name expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl SystemMessageExpression {
    /// Compiles the expression into a concrete [`SystemMessage`].
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
    /// Prepares the message by normalizing content and optional fields.
    pub fn prepare(&mut self) {
        self.content.prepare();
        if self.name.as_ref().is_some_and(String::is_empty) {
            self.name = None;
        }
    }
}

/// Expression variant of [`UserMessage`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageExpression {
    /// The message content expression.
    pub content: functions::expression::WithExpression<RichContentExpression>,
    /// Optional name expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<functions::expression::WithExpression<Option<String>>>,
}

impl UserMessageExpression {
    /// Compiles the expression into a concrete [`UserMessage`].
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

/// A tool message containing the result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
}

impl ToolMessage {
    /// Prepares the message by normalizing its content.
    pub fn prepare(&mut self) {
        self.content.prepare();
    }
}

/// Expression variant of [`ToolMessage`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMessageExpression {
    /// The content expression.
    pub content: functions::expression::WithExpression<RichContentExpression>,
    /// The tool call ID expression.
    pub tool_call_id: functions::expression::WithExpression<String>,
}

impl ToolMessageExpression {
    /// Compiles the expression into a concrete [`ToolMessage`].
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

/// An assistant message (model's previous response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The message content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RichContent>,
    /// Optional name for the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Refusal message if the model declined to respond.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantToolCall>>,
    /// Reasoning content from models that support chain-of-thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

impl AssistantMessage {
    /// Prepares the message by normalizing content and optional fields.
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

/// Expression variant of [`AssistantMessage`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessageExpression {
    /// The content expression.
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
    /// Compiles the expression into a concrete [`AssistantMessage`].
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

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCall {
    /// A function call with an ID and function details.
    Function {
        /// The unique ID of this tool call.
        id: String,
        /// The function being called.
        function: AssistantToolCallFunction,
    },
}

impl AssistantToolCall {
    /// Returns `true` if all fields are empty.
    pub fn is_empty(&self) -> bool {
        match self {
            AssistantToolCall::Function { id, function } => {
                id.is_empty() && function.is_empty()
            }
        }
    }
}

/// Expression variant of [`AssistantToolCall`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCallExpression {
    /// A function call expression.
    Function {
        /// The tool call ID expression.
        id: functions::expression::WithExpression<String>,
        /// The function expression.
        function: functions::expression::WithExpression<
            AssistantToolCallFunctionExpression,
        >,
    },
}

impl AssistantToolCallExpression {
    /// Compiles the expression into a concrete [`AssistantToolCall`].
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

/// Details of a function call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function, as a JSON string.
    pub arguments: String,
}

impl AssistantToolCallFunction {
    /// Returns `true` if both name and arguments are empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.arguments.is_empty()
    }
}

/// Expression variant of [`AssistantToolCallFunction`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantToolCallFunctionExpression {
    /// The function name expression.
    pub name: functions::expression::WithExpression<String>,
    /// The arguments expression.
    pub arguments: functions::expression::WithExpression<String>,
}

impl AssistantToolCallFunctionExpression {
    /// Compiles the expression into a concrete [`AssistantToolCallFunction`].
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
    /// Prepares the content by consolidating parts into a single text string.
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

/// Expression variant of [`SimpleContent`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SimpleContentExpression {
    /// Plain text content.
    Text(String),
    /// Multi-part text content expressions.
    Parts(
        Vec<functions::expression::WithExpression<SimpleContentPartExpression>>,
    ),
}

impl SimpleContentExpression {
    /// Compiles the expression into a concrete [`SimpleContent`].
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
                            compiled_parts.push(one_part.compile(params)?);
                        }
                        functions::expression::OneOrMany::Many(many_parts) => {
                            for part in many_parts {
                                compiled_parts.push(part.compile(params)?);
                            }
                        }
                    }
                }
                Ok(SimpleContent::Parts(compiled_parts))
            }
        }
    }
}

/// A part of simple text content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimpleContentPart {
    /// A text part.
    Text {
        /// The text content.
        text: String,
    },
}

/// Expression variant of [`SimpleContentPart`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimpleContentPartExpression {
    /// A text part expression.
    Text {
        /// The text expression.
        text: functions::expression::WithExpression<String>,
    },
}

impl SimpleContentPartExpression {
    /// Compiles the expression into a concrete [`SimpleContentPart`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<SimpleContentPart, functions::expression::ExpressionError> {
        match self {
            SimpleContentPartExpression::Text { text } => {
                let text = text.compile_one(params)?;
                Ok(SimpleContentPart::Text { text })
            }
        }
    }
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
    /// Prepares the content by normalizing parts.
    ///
    /// This consolidates consecutive text parts, removes empty parts,
    /// and converts single-part content to plain text.
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

    /// Returns `true` if the content is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            RichContent::Text(text) => text.is_empty(),
            RichContent::Parts(parts) => parts.is_empty(),
        }
    }

    /// Computes a content-addressed ID for this content.
    pub fn id(&self) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// Expression variant of [`RichContent`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RichContentExpression {
    /// Plain text content.
    Text(String),
    /// Multi-part content expressions.
    Parts(
        Vec<functions::expression::WithExpression<RichContentPartExpression>>,
    ),
}

impl RichContentExpression {
    /// Compiles the expression into a concrete [`RichContent`].
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
                            compiled_parts.push(one_part.compile(params)?);
                        }
                        functions::expression::OneOrMany::Many(many_parts) => {
                            for part in many_parts {
                                compiled_parts.push(part.compile(params)?);
                            }
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
    /// Prepares the content part by normalizing optional fields.
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

    /// Returns `true` if the content part is empty.
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

/// Expression variant of [`RichContentPart`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichContentPartExpression {
    Text {
        text: functions::expression::WithExpression<String>,
    },
    ImageUrl {
        image_url: functions::expression::WithExpression<ImageUrl>,
    },
    InputAudio {
        input_audio: functions::expression::WithExpression<InputAudio>,
    },
    InputVideo {
        video_url: functions::expression::WithExpression<VideoUrl>,
    },
    VideoUrl {
        video_url: functions::expression::WithExpression<VideoUrl>,
    },
    File {
        file: functions::expression::WithExpression<File>,
    },
}

impl RichContentPartExpression {
    /// Compiles the expression into a concrete [`RichContentPart`].
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<RichContentPart, functions::expression::ExpressionError> {
        match self {
            RichContentPartExpression::Text { text } => {
                let text = text.compile_one(params)?;
                Ok(RichContentPart::Text { text })
            }
            RichContentPartExpression::ImageUrl { image_url } => {
                let image_url = image_url.compile_one(params)?;
                Ok(RichContentPart::ImageUrl { image_url })
            }
            RichContentPartExpression::InputAudio { input_audio } => {
                let input_audio = input_audio.compile_one(params)?;
                Ok(RichContentPart::InputAudio { input_audio })
            }
            RichContentPartExpression::InputVideo { video_url } => {
                let video_url = video_url.compile_one(params)?;
                Ok(RichContentPart::InputVideo { video_url })
            }
            RichContentPartExpression::VideoUrl { video_url } => {
                let video_url = video_url.compile_one(params)?;
                Ok(RichContentPart::VideoUrl { video_url })
            }
            RichContentPartExpression::File { file } => {
                let file = file.compile_one(params)?;
                Ok(RichContentPart::File { file })
            }
        }
    }
}

/// An image URL for multimodal input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The URL of the image (can be a data URL or HTTP URL).
    pub url: String,
    /// The detail level for image processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageUrlDetail>,
}

impl ImageUrl {
    /// Prepares the image URL by normalizing the detail field.
    pub fn prepare(&mut self) {
        if matches!(self.detail, Some(ImageUrlDetail::Auto)) {
            self.detail = None;
        }
    }

    /// Returns `true` if the URL is empty and no detail is set.
    pub fn is_empty(&self) -> bool {
        self.url.is_empty() && self.detail.is_none()
    }
}

/// Detail level for image processing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageUrlDetail {
    /// Let the model decide the detail level.
    #[serde(rename = "auto")]
    Auto,
    /// Low detail mode (faster, less tokens).
    #[serde(rename = "low")]
    Low,
    /// High detail mode (more accurate, more tokens).
    #[serde(rename = "high")]
    High,
}

/// Audio input for multimodal messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputAudio {
    /// Base64-encoded audio data.
    pub data: String,
    /// The audio format (e.g., "wav", "mp3").
    pub format: String,
}

impl InputAudio {
    /// Returns `true` if both data and format are empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.format.is_empty()
    }
}

/// A video URL for multimodal input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoUrl {
    /// The URL of the video.
    pub url: String,
}

impl VideoUrl {
    /// Returns `true` if the URL is empty.
    pub fn is_empty(&self) -> bool {
        self.url.is_empty()
    }
}

/// A file attachment for multimodal input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// Base64-encoded file data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,
    /// The ID of a previously uploaded file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// The filename for display purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// A URL to fetch the file from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
}

impl File {
    /// Prepares the file by normalizing empty strings to `None`.
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

    /// Returns `true` if all file fields are `None`.
    pub fn is_empty(&self) -> bool {
        self.file_data.is_none()
            && self.file_id.is_none()
            && self.filename.is_none()
            && self.file_url.is_none()
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    #[serde(rename = "system")]
    System(SystemMessage),
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
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
pub struct DeveloperMessage {
    pub content: SimpleContent,
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
pub struct SystemMessage {
    pub content: SimpleContent,
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
pub struct UserMessage {
    pub content: RichContent,
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
#[serde(untagged)]
pub enum SimpleContent {
    Text(String),
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SimpleContentPart {
    Text { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RichContent {
    Text(String),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RichContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
    InputAudio { input_audio: InputAudio },
    InputVideo { video_url: VideoUrl },
    VideoUrl { video_url: VideoUrl },
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

//! Rich content types for user/assistant messages (supports multimodal input).

use crate::functions;
use functions::expression::{
    ExpressionError, FromStarlarkValue, ToStarlarkValue, WithExpression,
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use starlark::values::dict::{
    AllocDict as StarlarkAllocDict, DictRef as StarlarkDictRef,
};
use starlark::values::{
    Heap as StarlarkHeap, UnpackValue, Value as StarlarkValue,
};

/// Rich content for user/assistant messages (supports multimodal input).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.RichContent")]
pub enum RichContent {
    /// Plain text content.
    #[schemars(title = "Text")]
    Text(String),
    /// Multi-part content (text, images, audio, video, files).
    #[schemars(title = "Parts")]
    Parts(Vec<RichContentPart>),
}

impl RichContent {
    pub fn push(&mut self, other: &RichContent) {
        match (&mut *self, other) {
            (RichContent::Text(self_text), RichContent::Text(other_text)) => {
                self_text.push_str(&other_text);
            }
            (RichContent::Text(self_text), RichContent::Parts(other_parts)) => {
                let mut parts = Vec::with_capacity(1 + other_parts.len());
                parts.push(RichContentPart::Text {
                    text: std::mem::take(self_text),
                });
                parts.extend(other_parts.iter().cloned());
                *self = RichContent::Parts(parts);
            }
            (RichContent::Parts(self_parts), RichContent::Text(other_text)) => {
                self_parts.push(RichContentPart::Text {
                    text: other_text.clone(),
                });
            }
            (
                RichContent::Parts(self_parts),
                RichContent::Parts(other_parts),
            ) => {
                self_parts.extend(other_parts.iter().cloned());
            }
        }
    }

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

    /// Extracts every chunk of this content into its own on-disk
    /// log file, returning a [`super::RichContentLog`] of
    /// [`LogReference`]s pointing at the written files (no inline
    /// content — the log is purely references).
    ///
    /// Per-chunk write rules:
    ///
    /// - `RichContent::Text(text)` → one `.txt` file at
    ///   `<media_root>/text/<id>-<idx>.txt` containing the raw
    ///   UTF-8 text. Return `RichContentLog::Reference(ref)`.
    /// - `RichContent::Parts(parts)` → one file per part (rules
    ///   below in [`Self::extract_one_part`]). Return
    ///   `RichContentLog::Parts(vec_of_refs)` in original order.
    ///
    /// `media_root` is the parent directory under which the
    /// per-media-type subdirs (`text`, `image`, `audio`, `video`,
    /// `file`) get created. Callers pass things like
    /// `"agents/completions/request/messages"` for message extraction
    /// or `"agents/completions/request/notifications"` for notification
    /// extraction.
    ///
    /// `id` and `message_index` identify the parent record.
    #[cfg(feature = "filesystem")]
    pub fn extract_media(
        self,
        media_root: &str,
        id: &str,
        message_index: u64,
    ) -> (super::RichContentLog, Vec<crate::filesystem::logs::LogFile>) {
        use crate::filesystem::logs::{LogFile, LogReference};
        use super::RichContentLog;

        match self {
            RichContent::Text(text) => {
                let log_file = LogFile {
                    route: format!("{media_root}/text"),
                    id: id.to_string(),
                    message_index: Some(message_index),
                    media_index: None,
                    extension: "txt".to_string(),
                    content: text.into_bytes(),
                };
                let reference = LogReference::new(log_file.path());
                (RichContentLog::Reference(reference), vec![log_file])
            }
            RichContent::Parts(parts) => {
                let mut log_refs = Vec::with_capacity(parts.len());
                let mut files = Vec::with_capacity(parts.len());
                for (part_idx, part) in parts.into_iter().enumerate() {
                    let file = Self::extract_one_part(
                        part,
                        media_root,
                        id,
                        message_index,
                        part_idx as u64,
                    );
                    log_refs.push(LogReference::new(file.path()));
                    files.push(file);
                }
                (RichContentLog::Parts(log_refs), files)
            }
        }
    }

    /// Write one `RichContentPart` to its own [`LogFile`].
    ///
    /// File-type choice per part:
    /// - `Text { text }` → `.txt` (raw UTF-8) under `<media_root>/text/`.
    /// - Inline-decodable media (`ImageUrl` / `InputAudio` /
    ///   `InputVideo` / `VideoUrl` / `File` whose `file_content()`
    ///   yields a `FileContent` that successfully `decode()`s) → the
    ///   decoded binary written under `<media_root>/<media_dir>/`
    ///   with the `FileContent`'s native extension.
    /// - Anything else (remote URLs, non-decodable inline data) →
    ///   `serde_json::to_vec_pretty(&part)` written under
    ///   `<media_root>/<media_dir>/` as a `.json` file. The reader
    ///   can `serde_json::from_slice::<RichContentPart>` it back.
    ///
    /// `media_dir` is debug grouping only — `image|audio|video|file`
    /// per variant. Not load-bearing for parsing (the reader keys
    /// off the file extension).
    #[cfg(feature = "filesystem")]
    fn extract_one_part(
        part: RichContentPart,
        media_root: &str,
        id: &str,
        message_index: u64,
        part_idx: u64,
    ) -> crate::filesystem::logs::LogFile {
        use crate::filesystem::logs::LogFile;

        // Text branches out early — never goes through file_content().
        if let RichContentPart::Text { text } = &part {
            return LogFile {
                route: format!("{media_root}/text"),
                id: id.to_string(),
                message_index: Some(message_index),
                media_index: Some(part_idx),
                extension: "txt".to_string(),
                content: text.clone().into_bytes(),
            };
        }

        let (media_dir, bin_attempt) = match &part {
            RichContentPart::Text { .. } => unreachable!("handled above"),
            RichContentPart::ImageUrl { image_url } => ("image", image_url.file_content()),
            RichContentPart::InputAudio { input_audio } => ("audio", input_audio.file_content()),
            RichContentPart::InputVideo { video_url }
            | RichContentPart::VideoUrl { video_url } => ("video", video_url.file_content()),
            RichContentPart::File { file } => ("file", file.file_content()),
        };

        if let Some(fc) = bin_attempt {
            if let Ok(decoded) = fc.decode() {
                return LogFile {
                    route: format!("{media_root}/{media_dir}"),
                    id: id.to_string(),
                    message_index: Some(message_index),
                    media_index: Some(part_idx),
                    extension: fc.extension.to_string(),
                    content: decoded,
                };
            }
        }

        // Fallback: serialize the part itself (covers remote URLs +
        // any non-decodable inline data).
        let json = serde_json::to_vec_pretty(&part)
            .expect("RichContentPart serializes");
        LogFile {
            route: format!("{media_root}/{media_dir}"),
            id: id.to_string(),
            message_index: Some(message_index),
            media_index: Some(part_idx),
            extension: "json".to_string(),
            content: json,
        }
    }

    /// Computes a content-addressed ID for this content.
    pub fn id(&self) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(self).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }

    /// Validates that this content contains only text or image parts.
    ///
    /// Used by upstream agent definitions whose prefix/suffix content
    /// rendering can only express text and image media (audio, video, and
    /// file parts have no representation in those upstreams' prompts).
    /// Returns `Err` naming the offending part variant if any non-text /
    /// non-image part is present.
    pub fn validate_text_or_image_only(&self) -> Result<(), String> {
        match self {
            RichContent::Text(_) => Ok(()),
            RichContent::Parts(parts) => {
                for (idx, part) in parts.iter().enumerate() {
                    match part {
                        RichContentPart::Text { .. }
                        | RichContentPart::ImageUrl { .. } => {}
                        RichContentPart::InputAudio { .. } => {
                            return Err(format!(
                                "part[{idx}] has unsupported media type `input_audio`; only text and image parts are allowed"
                            ));
                        }
                        RichContentPart::InputVideo { .. } => {
                            return Err(format!(
                                "part[{idx}] has unsupported media type `input_video`; only text and image parts are allowed"
                            ));
                        }
                        RichContentPart::VideoUrl { .. } => {
                            return Err(format!(
                                "part[{idx}] has unsupported media type `video_url`; only text and image parts are allowed"
                            ));
                        }
                        RichContentPart::File { .. } => {
                            return Err(format!(
                                "part[{idx}] has unsupported media type `file`; only text and image parts are allowed"
                            ));
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

impl FromStarlarkValue for RichContent {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(RichContent::Text(s.to_owned()));
        }
        let parts = Vec::<RichContentPart>::from_starlark_value(value)?;
        Ok(RichContent::Parts(parts))
    }
}

/// Collapse a `Vec<RichContentPart>` into `RichContent`, joining
/// consecutive text-only parts into one `RichContent::Text` (separated
/// by `\n\n`) and leaving mixed-media inputs as `RichContent::Parts`.
/// Empty input yields `RichContent::Text(String::new())`.
impl From<Vec<RichContentPart>> for RichContent {
    fn from(parts: Vec<RichContentPart>) -> Self {
        if parts.is_empty() {
            return RichContent::Text(String::new());
        }
        let all_text = parts
            .iter()
            .all(|p| matches!(p, RichContentPart::Text { .. }));
        if all_text {
            let joined = parts
                .into_iter()
                .filter_map(|p| match p {
                    RichContentPart::Text { text } => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            RichContent::Text(joined)
        } else {
            RichContent::Parts(parts)
        }
    }
}

/// Convert an MCP `ContentBlock` into a `RichContentPart`. Lossless
/// for text / image / audio; `ResourceLink` and `EmbeddedResource`
/// fall back to a JSON-serialized text part so the content survives
/// even when there's no rich representation. Mirrors the resource-
/// resolution-free path of [`crate::mcp::Connection::call_tool_as_message`]
/// (the connection-bound method does extra work to fetch resource
/// contents, which this stateless `From` impl cannot do — callers
/// that want resource resolution must do it before invoking this
/// conversion).
#[cfg(feature = "mcp")]
impl From<crate::mcp::tool::ContentBlock> for RichContentPart {
    fn from(block: crate::mcp::tool::ContentBlock) -> Self {
        use crate::mcp::tool::ContentBlock;
        match block {
            ContentBlock::Text(t) => RichContentPart::Text { text: t.text },
            ContentBlock::Image(i) => RichContentPart::ImageUrl {
                image_url: i.into(),
            },
            ContentBlock::Audio(a) => RichContentPart::InputAudio {
                input_audio: a.into(),
            },
            block @ (ContentBlock::ResourceLink(_)
            | ContentBlock::EmbeddedResource(_)) => RichContentPart::Text {
                text: serde_json::to_string(&block).unwrap_or_default(),
            },
        }
    }
}

/// Build a `RichContent` from an MCP `Vec<ContentBlock>` via the
/// element-wise [`From<ContentBlock>`] impl, then collapse to plain
/// text when every part is text. Matches the shape produced by
/// `call_tool_as_message` and `build_drain_user_message` on the
/// agent side.
#[cfg(feature = "mcp")]
impl From<Vec<crate::mcp::tool::ContentBlock>> for RichContent {
    fn from(blocks: Vec<crate::mcp::tool::ContentBlock>) -> Self {
        let parts: Vec<RichContentPart> =
            blocks.into_iter().map(Into::into).collect();
        RichContent::from(parts)
    }
}

/// Expression variant of [`RichContent`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.message.RichContentExpression")]
pub enum RichContentExpression {
    /// Plain text content.
    #[schemars(title = "Text")]
    Text(String),
    /// Multi-part content expressions.
    #[schemars(title = "Parts")]
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

impl From<RichContent> for RichContentExpression {
    fn from(content: RichContent) -> Self {
        match content {
            RichContent::Text(text) => RichContentExpression::Text(text),
            RichContent::Parts(parts) => RichContentExpression::Parts(
                parts
                    .into_iter()
                    .map(RichContentPartExpression::from)
                    .map(WithExpression::Value)
                    .collect(),
            ),
        }
    }
}

impl FromStarlarkValue for RichContentExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        if let Ok(Some(s)) = <&str as UnpackValue>::unpack_value(*value) {
            return Ok(RichContentExpression::Text(s.to_owned()));
        }
        let parts = Vec::<WithExpression<RichContentPartExpression>>::from_starlark_value(value)?;
        Ok(RichContentExpression::Parts(parts))
    }
}

/// A part of rich content.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.RichContentPart")]
pub enum RichContentPart {
    /// Text content.
    #[schemars(title = "Text")]
    Text { text: String },
    /// An image URL.
    #[schemars(title = "ImageUrl")]
    ImageUrl { image_url: ImageUrl },
    /// Audio input.
    #[schemars(title = "InputAudio")]
    InputAudio { input_audio: InputAudio },
    /// Video input.
    #[schemars(title = "InputVideo")]
    InputVideo { video_url: VideoUrl },
    /// A video URL.
    #[schemars(title = "VideoUrl")]
    VideoUrl { video_url: VideoUrl },
    /// A file.
    #[schemars(title = "File")]
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

impl ToStarlarkValue for RichContentPart {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        match self {
            RichContentPart::Text { text } => heap.alloc(StarlarkAllocDict([
                ("type", "text".to_starlark_value(heap)),
                ("text", text.to_starlark_value(heap)),
            ])),
            RichContentPart::ImageUrl { image_url } => {
                heap.alloc(StarlarkAllocDict([
                    ("type", "image_url".to_starlark_value(heap)),
                    ("image_url", image_url.to_starlark_value(heap)),
                ]))
            }
            RichContentPart::InputAudio { input_audio } => {
                heap.alloc(StarlarkAllocDict([
                    ("type", "input_audio".to_starlark_value(heap)),
                    ("input_audio", input_audio.to_starlark_value(heap)),
                ]))
            }
            RichContentPart::InputVideo { video_url } => {
                heap.alloc(StarlarkAllocDict([
                    ("type", "input_video".to_starlark_value(heap)),
                    ("video_url", video_url.to_starlark_value(heap)),
                ]))
            }
            RichContentPart::VideoUrl { video_url } => {
                heap.alloc(StarlarkAllocDict([
                    ("type", "video_url".to_starlark_value(heap)),
                    ("video_url", video_url.to_starlark_value(heap)),
                ]))
            }
            RichContentPart::File { file } => heap.alloc(StarlarkAllocDict([
                ("type", "file".to_starlark_value(heap)),
                ("file", file.to_starlark_value(heap)),
            ])),
        }
    }
}

impl FromStarlarkValue for RichContentPart {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "RichContentPart: expected dict".into(),
            )
        })?;
        // First pass: find the type
        let mut typ = None;
        for (k, v) in dict.iter() {
            if let Ok(Some("type")) = <&str as UnpackValue>::unpack_value(k) {
                typ = Some(
                    <&str as UnpackValue>::unpack_value(v)
                        .map_err(|e| {
                            ExpressionError::StarlarkConversionError(
                                e.to_string(),
                            )
                        })?
                        .ok_or_else(|| {
                            ExpressionError::StarlarkConversionError(
                                "RichContentPart: expected string type".into(),
                            )
                        })?,
                );
                break;
            }
        }
        let typ = typ.ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "RichContentPart: missing type".into(),
            )
        })?;
        // Second pass: find the payload by expected key
        let payload_key = match typ {
            "text" => "text",
            "image_url" => "image_url",
            "input_audio" => "input_audio",
            "input_video" | "video_url" => "video_url",
            "file" => "file",
            _ => {
                return Err(ExpressionError::StarlarkConversionError(format!(
                    "RichContentPart: unknown type: {}",
                    typ
                )));
            }
        };
        let mut payload = None;
        for (k, v) in dict.iter() {
            if let Ok(Some(key)) = <&str as UnpackValue>::unpack_value(k) {
                if key == payload_key {
                    payload = Some(v);
                    break;
                }
            }
        }
        let v = payload.ok_or_else(|| {
            ExpressionError::StarlarkConversionError(format!(
                "RichContentPart: missing {}",
                payload_key
            ))
        })?;
        match typ {
            "text" => Ok(RichContentPart::Text {
                text: String::from_starlark_value(&v)?,
            }),
            "image_url" => Ok(RichContentPart::ImageUrl {
                image_url: ImageUrl::from_starlark_value(&v)?,
            }),
            "input_audio" => Ok(RichContentPart::InputAudio {
                input_audio: InputAudio::from_starlark_value(&v)?,
            }),
            "input_video" => Ok(RichContentPart::InputVideo {
                video_url: VideoUrl::from_starlark_value(&v)?,
            }),
            "video_url" => Ok(RichContentPart::VideoUrl {
                video_url: VideoUrl::from_starlark_value(&v)?,
            }),
            "file" => Ok(RichContentPart::File {
                file: File::from_starlark_value(&v)?,
            }),
            _ => unreachable!(),
        }
    }
}

/// Expression variant of [`RichContentPart`] for dynamic content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.message.RichContentPartExpression")]
pub enum RichContentPartExpression {
    #[schemars(title = "Text")]
    Text {
        text: functions::expression::WithExpression<String>,
    },
    #[schemars(title = "ImageUrl")]
    ImageUrl {
        image_url: functions::expression::WithExpression<ImageUrl>,
    },
    #[schemars(title = "InputAudio")]
    InputAudio {
        input_audio: functions::expression::WithExpression<InputAudio>,
    },
    #[schemars(title = "InputVideo")]
    InputVideo {
        video_url: functions::expression::WithExpression<VideoUrl>,
    },
    #[schemars(title = "VideoUrl")]
    VideoUrl {
        video_url: functions::expression::WithExpression<VideoUrl>,
    },
    #[schemars(title = "File")]
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

impl From<RichContentPart> for RichContentPartExpression {
    fn from(part: RichContentPart) -> Self {
        match part {
            RichContentPart::Text { text } => RichContentPartExpression::Text {
                text: WithExpression::Value(text),
            },
            RichContentPart::ImageUrl { image_url } => {
                RichContentPartExpression::ImageUrl {
                    image_url: WithExpression::Value(image_url),
                }
            }
            RichContentPart::InputAudio { input_audio } => {
                RichContentPartExpression::InputAudio {
                    input_audio: WithExpression::Value(input_audio),
                }
            }
            RichContentPart::InputVideo { video_url } => {
                RichContentPartExpression::InputVideo {
                    video_url: WithExpression::Value(video_url),
                }
            }
            RichContentPart::VideoUrl { video_url } => {
                RichContentPartExpression::VideoUrl {
                    video_url: WithExpression::Value(video_url),
                }
            }
            RichContentPart::File { file } => RichContentPartExpression::File {
                file: WithExpression::Value(file),
            },
        }
    }
}

impl FromStarlarkValue for RichContentPartExpression {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let part = RichContentPart::from_starlark_value(value)?;
        match part {
            RichContentPart::Text { text } => {
                Ok(RichContentPartExpression::Text {
                    text: WithExpression::Value(text),
                })
            }
            RichContentPart::ImageUrl { image_url } => {
                Ok(RichContentPartExpression::ImageUrl {
                    image_url: WithExpression::Value(image_url),
                })
            }
            RichContentPart::InputAudio { input_audio } => {
                Ok(RichContentPartExpression::InputAudio {
                    input_audio: WithExpression::Value(input_audio),
                })
            }
            RichContentPart::InputVideo { video_url } => {
                Ok(RichContentPartExpression::InputVideo {
                    video_url: WithExpression::Value(video_url),
                })
            }
            RichContentPart::VideoUrl { video_url } => {
                Ok(RichContentPartExpression::VideoUrl {
                    video_url: WithExpression::Value(video_url),
                })
            }
            RichContentPart::File { file } => {
                Ok(RichContentPartExpression::File {
                    file: WithExpression::Value(file),
                })
            }
        }
    }
}

/// An image URL for multimodal input.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.ImageUrl")]
pub struct ImageUrl {
    /// The URL of the image (can be a data URL or HTTP URL).
    pub url: String,
    /// The detail level for image processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
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

    /// Returns extractable file content if this is a base64 data URL.
    ///
    /// HTTP/HTTPS URLs return `None` (kept inline).
    pub fn file_content(&self) -> Option<super::FileContent<'_>> {
        let (mime, payload) = super::file_content::parse_data_url(&self.url)?;
        Some(super::FileContent {
            content: payload,
            extension: super::file_content::mime_to_ext(mime),
        })
    }
}

/// Compose a base64 data URL from an MCP `ImageContent`'s mime + data.
/// `detail` defaults to `None`.
#[cfg(feature = "mcp")]
impl From<crate::mcp::tool::ImageContent> for ImageUrl {
    fn from(image: crate::mcp::tool::ImageContent) -> Self {
        Self {
            url: format!("data:{};base64,{}", image.mime_type, image.data),
            detail: None,
        }
    }
}

impl ToStarlarkValue for ImageUrl {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        heap.alloc(StarlarkAllocDict([
            ("url", self.url.to_starlark_value(heap)),
            ("detail", self.detail.to_starlark_value(heap)),
        ]))
    }
}

impl FromStarlarkValue for ImageUrl {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "ImageUrl: expected dict".into(),
            )
        })?;
        let mut url = None;
        let mut detail = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "ImageUrl: expected string key".into(),
                    )
                })?;
            match key {
                "url" => url = Some(String::from_starlark_value(&v)?),
                "detail" => {
                    detail = Option::<ImageUrlDetail>::from_starlark_value(&v)?
                }
                _ => {}
            }
            if url.is_some() && detail.is_some() {
                break;
            }
        }
        Ok(ImageUrl {
            url: url.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ImageUrl: missing url".into(),
                )
            })?,
            detail,
        })
    }
}

/// Detail level for image processing.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.ImageUrlDetail")]
pub enum ImageUrlDetail {
    /// Let the model decide the detail level.
    #[schemars(title = "Auto")]
    #[serde(rename = "auto")]
    Auto,
    /// Low detail mode (faster, less tokens).
    #[schemars(title = "Low")]
    #[serde(rename = "low")]
    Low,
    /// High detail mode (more accurate, more tokens).
    #[schemars(title = "High")]
    #[serde(rename = "high")]
    High,
}

impl ToStarlarkValue for ImageUrlDetail {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        match self {
            ImageUrlDetail::Auto => "auto".to_starlark_value(heap),
            ImageUrlDetail::Low => "low".to_starlark_value(heap),
            ImageUrlDetail::High => "high".to_starlark_value(heap),
        }
    }
}

impl FromStarlarkValue for ImageUrlDetail {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let s = <&str as UnpackValue>::unpack_value(*value)
            .map_err(|e| {
                ExpressionError::StarlarkConversionError(e.to_string())
            })?
            .ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "ImageUrlDetail: expected string".into(),
                )
            })?;
        match s {
            "auto" => Ok(ImageUrlDetail::Auto),
            "low" => Ok(ImageUrlDetail::Low),
            "high" => Ok(ImageUrlDetail::High),
            _ => Err(ExpressionError::StarlarkConversionError(format!(
                "ImageUrlDetail: unknown value: {}",
                s
            ))),
        }
    }
}

/// Audio input for multimodal messages.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.InputAudio")]
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

    /// Returns extractable file content if audio data is present.
    ///
    /// Audio is always base64-encoded inline, so this returns `Some`
    /// whenever `data` is non-empty.
    pub fn file_content(&self) -> Option<super::FileContent<'_>> {
        if self.data.is_empty() {
            return None;
        }
        Some(super::FileContent {
            content: &self.data,
            extension: if self.format.is_empty() { "bin" } else { &self.format },
        })
    }
}

/// Adopt an MCP `AudioContent`'s `mime_type` as `format` verbatim.
#[cfg(feature = "mcp")]
impl From<crate::mcp::tool::AudioContent> for InputAudio {
    fn from(audio: crate::mcp::tool::AudioContent) -> Self {
        Self {
            data: audio.data,
            format: audio.mime_type,
        }
    }
}

impl ToStarlarkValue for InputAudio {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        heap.alloc(StarlarkAllocDict([
            ("data", self.data.to_starlark_value(heap)),
            ("format", self.format.to_starlark_value(heap)),
        ]))
    }
}

impl FromStarlarkValue for InputAudio {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "InputAudio: expected dict".into(),
            )
        })?;
        let mut data = None;
        let mut format = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "InputAudio: expected string key".into(),
                    )
                })?;
            match key {
                "data" => data = Some(String::from_starlark_value(&v)?),
                "format" => format = Some(String::from_starlark_value(&v)?),
                _ => {}
            }
            if data.is_some() && format.is_some() {
                break;
            }
        }
        Ok(InputAudio {
            data: data.unwrap_or_default(),
            format: format.unwrap_or_default(),
        })
    }
}

/// A video URL for multimodal input.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.VideoUrl")]
pub struct VideoUrl {
    /// The URL of the video.
    pub url: String,
}

impl VideoUrl {
    /// Returns `true` if the URL is empty.
    pub fn is_empty(&self) -> bool {
        self.url.is_empty()
    }

    /// Returns extractable file content if this is a base64 data URL.
    ///
    /// HTTP/HTTPS URLs return `None` (kept inline).
    pub fn file_content(&self) -> Option<super::FileContent<'_>> {
        let (mime, payload) = super::file_content::parse_data_url(&self.url)?;
        Some(super::FileContent {
            content: payload,
            extension: super::file_content::mime_to_ext(mime),
        })
    }
}

impl ToStarlarkValue for VideoUrl {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        heap.alloc(StarlarkAllocDict([(
            "url",
            self.url.to_starlark_value(heap),
        )]))
    }
}

impl FromStarlarkValue for VideoUrl {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "VideoUrl: expected dict".into(),
            )
        })?;
        let mut url = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "VideoUrl: expected string key".into(),
                    )
                })?;
            if key == "url" {
                url = Some(String::from_starlark_value(&v)?);
            }
        }
        Ok(VideoUrl {
            url: url.ok_or_else(|| {
                ExpressionError::StarlarkConversionError(
                    "VideoUrl: missing url".into(),
                )
            })?,
        })
    }
}

/// A file attachment for multimodal input.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.message.File")]
pub struct File {
    /// Base64-encoded file data.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub file_data: Option<String>,
    /// The ID of a previously uploaded file.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub file_id: Option<String>,
    /// The filename for display purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub filename: Option<String>,
    /// A URL to fetch the file from.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
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

    /// Returns extractable file content if inline file data is present.
    ///
    /// Files referenced only by URL or ID return `None` (kept inline).
    pub fn file_content(&self) -> Option<super::FileContent<'_>> {
        let data = self.file_data.as_deref()?;
        if data.is_empty() {
            return None;
        }
        let ext = self.filename.as_deref()
            .and_then(|name| name.rsplit_once('.'))
            .map(|(_, ext)| ext)
            .unwrap_or("bin");
        Some(super::FileContent {
            content: data,
            extension: ext,
        })
    }
}

impl ToStarlarkValue for File {
    fn to_starlark_value<'v>(
        &self,
        heap: &'v StarlarkHeap,
    ) -> StarlarkValue<'v> {
        heap.alloc(StarlarkAllocDict([
            ("file_data", self.file_data.to_starlark_value(heap)),
            ("file_id", self.file_id.to_starlark_value(heap)),
            ("filename", self.filename.to_starlark_value(heap)),
            ("file_url", self.file_url.to_starlark_value(heap)),
        ]))
    }
}

impl FromStarlarkValue for File {
    fn from_starlark_value(
        value: &StarlarkValue,
    ) -> Result<Self, ExpressionError> {
        let dict = StarlarkDictRef::from_value(*value).ok_or_else(|| {
            ExpressionError::StarlarkConversionError(
                "File: expected dict".into(),
            )
        })?;
        let mut file_data = None;
        let mut file_id = None;
        let mut filename = None;
        let mut file_url = None;
        for (k, v) in dict.iter() {
            let key = <&str as UnpackValue>::unpack_value(k)
                .map_err(|e| {
                    ExpressionError::StarlarkConversionError(e.to_string())
                })?
                .ok_or_else(|| {
                    ExpressionError::StarlarkConversionError(
                        "File: expected string key".into(),
                    )
                })?;
            match key {
                "file_data" => {
                    file_data = Option::<String>::from_starlark_value(&v)?
                }
                "file_id" => {
                    file_id = Option::<String>::from_starlark_value(&v)?
                }
                "filename" => {
                    filename = Option::<String>::from_starlark_value(&v)?
                }
                "file_url" => {
                    file_url = Option::<String>::from_starlark_value(&v)?
                }
                _ => {}
            }
        }
        Ok(File {
            file_data,
            file_id,
            filename,
            file_url,
        })
    }
}

crate::functions::expression::impl_from_special_unsupported!(
    RichContentExpression,
    RichContentPartExpression,
    ImageUrl,
    InputAudio,
    VideoUrl,
    File,
);

impl crate::functions::expression::FromSpecial
    for Vec<crate::functions::expression::WithExpression<RichContentExpression>>
{
    fn from_special(
        _special: &crate::functions::expression::Special,
        _params: &crate::functions::expression::Params,
    ) -> Result<Self, crate::functions::expression::ExpressionError> {
        Err(crate::functions::expression::ExpressionError::UnsupportedSpecial)
    }
}

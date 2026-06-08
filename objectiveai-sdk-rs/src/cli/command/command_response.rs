/// Convert a typed CLI response item into its MCP-shaped projection —
/// either a media [`ContentBlock`] for MCP tool results or a JSONL
/// `serde_json::Value` for the line-oriented stream wire format.
/// Implementors are the per-leaf response shapes in the surrounding
/// tree (`agents::spawn::ResponseItem`, `tools::run::ResponseItem`, …)
/// — not yet wired up; the trait exists to anchor the contract.
#[cfg(feature = "mcp")]
pub trait CommandResponse {
    fn into_mcp(self) -> McpResponseItem;
}

/// MCP-side projection of one CLI response item.
///
/// - [`McpResponseItem::Media`] carries a typed [`ContentBlock`] for
///   image / audio / file payloads that ride the MCP tool result as
///   real media blocks.
/// - [`McpResponseItem::JSONL`] carries a `serde_json::Value` for the
///   line-oriented JSONL wire format every other leaf emits.
///
/// Not serde-shaped on purpose: this is the runtime carrier between
/// `CommandResponse::into_mcp` and the formatter; nothing on either
/// side needs to round-trip it through JSON.
#[cfg(feature = "mcp")]
#[derive(Debug, Clone)]
pub enum McpResponseItem {
    Media(crate::mcp::tool::ContentBlock),
    JSONL(serde_json::Value),
}

// Bare-media leaf impls. Every log media leaf under
// `cli/command/logs/agents/completions/{request,response}/messages/.../{get,subscribe}`
// declares `pub type Response = <media type>` for one of the four media
// types below, so these four impls cover all 20 such leaves through the
// type aliases. Each `self.into()` reaches the matching
// `From<T> for ContentBlock` impl in `mcp/tool/content_block.rs`.

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::ImageUrl {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::Media(self.into())
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::InputAudio {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::Media(self.into())
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::VideoUrl {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::Media(self.into())
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::File {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::Media(self.into())
    }
}

// `RichContentPart` is a sum over the four media types above plus
// `Text(String)`. Used directly as the `agents queue read id` leaf's
// `Response` type so the queue-content reader's wire shape is bit-
// identical to a rich-content part. Each arm delegates to the inner
// type's impl: media variants pick up
// `McpResponseItem::Media(ContentBlock::…)`, `Text` picks up
// `Value::String` via the `String` impl below.
#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::RichContentPart {
    fn into_mcp(self) -> McpResponseItem {
        use crate::agent::completions::message::RichContentPart;
        match self {
            RichContentPart::Text { text } => text.into_mcp(),
            RichContentPart::ImageUrl { image_url } => image_url.into_mcp(),
            RichContentPart::InputAudio { input_audio } => input_audio.into_mcp(),
            RichContentPart::InputVideo { video_url } => video_url.into_mcp(),
            RichContentPart::VideoUrl { video_url } => video_url.into_mcp(),
            RichContentPart::File { file } => file.into_mcp(),
        }
    }
}

// JSONL passthrough — `serde_json::to_value(self)` is the body for
// every alias target whose leaf simply emits its serde-shaped value
// as one JSONL line. Special-cased shortcuts: `String` constructs
// `Value::String` directly; `serde_json::Value` rides through as-is;
// `Option<T>` delegates to `T::into_mcp` on `Some` and emits a JSONL
// null on `None`.

#[cfg(feature = "mcp")]
impl CommandResponse for String {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(serde_json::Value::String(self))
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for serde_json::Value {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(self)
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for schemars::Schema {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::cli::command::ResponseSchema {
    fn into_mcp(self) -> McpResponseItem {
        self.0.into_mcp()
    }
}

#[cfg(feature = "mcp")]
impl<T: CommandResponse> CommandResponse for Option<T> {
    fn into_mcp(self) -> McpResponseItem {
        match self {
            Some(v) => v.into_mcp(),
            None => McpResponseItem::JSONL(serde_json::Value::Null),
        }
    }
}

#[cfg(feature = "mcp")]
impl<T: CommandResponse> CommandResponse for Result<T, crate::cli::Error> {
    fn into_mcp(self) -> McpResponseItem {
        match self {
            Ok(v) => v.into_mcp(),
            Err(e) => e.into_mcp(),
        }
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::AssistantToolCallDelta {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::message::AssistantToolCall {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::completions::response::Logprobs {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::agent::response::GetAgentResponse {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::functions::profiles::response::GetProfileResponse {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::functions::response::GetFunctionResponse {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::Remote {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::swarm::response::GetSwarmResponse {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}

#[cfg(feature = "mcp")]
impl CommandResponse for crate::vector::completions::request::VectorCompletionCreateParams {
    fn into_mcp(self) -> McpResponseItem {
        McpResponseItem::JSONL(
            serde_json::to_value(self).unwrap(),
        )
    }
}


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

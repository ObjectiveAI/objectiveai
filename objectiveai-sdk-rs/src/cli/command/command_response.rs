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

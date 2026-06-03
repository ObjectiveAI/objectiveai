//! `Vec<McpResponseItem>` → `Vec<rmcp::model::Content>` formatter.
//!
//! Three output modes, picked by item count:
//!
//! - **0 items** → the `<empty>` sentinel block.
//! - **1 item** → the item raw, no surrounding `[…]`, no quotes, no
//!   escaping. The response *is* the item.
//! - **N ≥ 2 items** → JSON-array-of-strings encoding where the
//!   concatenation of *text-content* blocks parses as `Vec<Value>`.
//!   Media + raw strings emit a `"`, body, `"` triplet so the body is
//!   its own addressable block; non-string JSON values emit one
//!   `serde_json::to_string` block since they're already self-
//!   delimiting JSON tokens.
//!
//! Text-carrier media (`ContentBlock::Text` produced by the bridge,
//! e.g. a remote ImageUrl encoded as a Text block with kind marker)
//! is *not* json-escaped — the body is media content (data URL, remote
//! URL, etc.) and escaping it could corrupt it. Plain raw strings
//! (`Value::String`) *are* escaped since they're arbitrary text.

use objectiveai_sdk::cli::command::McpResponseItem;
use objectiveai_sdk::mcp::tool::ContentBlock;
use rmcp::model::Content;
use serde_json::Value;

use crate::bridge::into_rmcp_content;

/// Format `items` into the MCP tool response `Vec<Content>`. See
/// module-level docs for the three output modes.
pub fn format_items(items: Vec<McpResponseItem>) -> Vec<Content> {
    match items.len() {
        0 => vec![Content::text("<empty>")],
        1 => {
            // Single-item raw form: no array brackets, no quoting, no
            // escaping. The consumer reads the item as-is.
            let item = items.into_iter().next().unwrap();
            match item {
                McpResponseItem::Media(ContentBlock::Text(t)) => {
                    vec![Content::text(t.text)]
                }
                McpResponseItem::Media(other) => vec![into_rmcp_content(other)],
                McpResponseItem::JSONL(Value::String(s)) => vec![Content::text(s)],
                McpResponseItem::JSONL(other) => {
                    let body = serde_json::to_string(&other)
                        .unwrap_or_else(|_| String::from("<serialize error>"));
                    vec![Content::text(body)]
                }
            }
        }
        _ => {
            // Capacity heuristic: most elements emit one block;
            // strings + media emit three. Overshoot to avoid reallocs.
            let mut blocks: Vec<Content> = Vec::with_capacity(items.len() * 3 + 2);
            blocks.push(Content::text("["));
            let mut first = true;
            for item in items {
                if !first {
                    blocks.push(Content::text(","));
                }
                first = false;
                match item {
                    // Text-carrier media — `"`, raw body, `"`. NOT
                    // json-escaped: the body is media content (e.g. a
                    // data URL) and escaping it could break it.
                    McpResponseItem::Media(ContentBlock::Text(t)) => {
                        blocks.push(Content::text("\""));
                        blocks.push(Content::text(t.text));
                        blocks.push(Content::text("\""));
                    }
                    // Real media — `"`, bridged Content, `"`. The
                    // strip-media projection collapses to `""` for
                    // this slot, keeping the array parseable as
                    // Vec<Value>.
                    McpResponseItem::Media(other) => {
                        blocks.push(Content::text("\""));
                        blocks.push(into_rmcp_content(other));
                        blocks.push(Content::text("\""));
                    }
                    // Raw JSON string — `"`, json-escaped body, `"`.
                    // Body is its own addressable block so a consumer
                    // can extract it without parsing JSON.
                    McpResponseItem::JSONL(Value::String(s)) => {
                        blocks.push(Content::text("\""));
                        blocks.push(Content::text(format!(
                            "{}",
                            json_escape::escape_str(&s)
                        )));
                        blocks.push(Content::text("\""));
                    }
                    // Typed JSON value (object / array / number / bool
                    // / null) — already a self-delimiting JSON token,
                    // so one block holds the full serialized value.
                    McpResponseItem::JSONL(other) => {
                        let body = serde_json::to_string(&other)
                            .unwrap_or_else(|_| String::from("\"<serialize error>\""));
                        blocks.push(Content::text(body));
                    }
                }
            }
            blocks.push(Content::text("]"));
            blocks
        }
    }
}

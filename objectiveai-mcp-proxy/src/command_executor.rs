//! The proxy's REAL command executor for server-side plugin upstreams.
//!
//! When an HTTP upstream is marked as a plugin by the typed
//! `X-MCP-Plugins` header, the proxy connects it with THIS executor
//! instead of the SDK's default not-supported one: a `cli_request`
//! pushed by the plugin's MCP server is fulfilled by forwarding the
//! command to the CLI daemon over the reverse channel
//! ([`ReverseChannel::command`]) and streaming the daemon's response
//! items back AS THEY ARRIVE. (Today all plugins are client-side
//! `ws://` upstreams, so this path is future-proofing for server-side
//! plugins.)

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use objectiveai_sdk::cli::command::{AgentArguments, Request, ResponseItem};
use objectiveai_sdk::client_objectiveai_mcp::server_response::CommandFrame;
use objectiveai_sdk::mcp::server::Plugin;
use objectiveai_sdk::mcp::{Error as McpError, McpClientCommandExecutor};
use tokio::sync::RwLock;

use crate::reverse_channel::{ReverseChannel, transport_error};

/// Fulfills a server-side plugin's command requests over the reverse
/// channel.
///
/// The daemon requires BOTH identities on every request:
/// - `plugin`: this upstream's four coordinates, from the typed
///   `X-MCP-Plugins` marker — the daemon stamps the trio on the
///   command's scope so the plugin run-gates apply.
/// - agent arguments: read from the SHARED session transient bag AT
///   `execute()` TIME (the bag full-replaces on every `initialize`,
///   so a connect-time snapshot would go stale).
#[derive(Clone)]
pub struct ReverseChannelCommandExecutor {
    pub(crate) channel: ReverseChannel,
    pub(crate) plugin: Plugin,
    /// The SAME allocation as `Session::transient_headers`.
    pub(crate) transient: Arc<RwLock<IndexMap<String, String>>>,
    /// Bounds only the wait for the exchange's first frame (the ack);
    /// items stream with no artificial deadline.
    pub(crate) ack_timeout: Option<Duration>,
}

impl McpClientCommandExecutor for ReverseChannelCommandExecutor {
    type Error = McpError;
    type Stream =
        Pin<Box<dyn Stream<Item = Result<ResponseItem, McpError>> + Send>>;

    async fn execute(
        &self,
        request: Request,
    ) -> Result<Self::Stream, Self::Error> {
        let agent_arguments = AgentArguments::from_transient_headers(
            &*self.transient.read().await,
        );
        let frames = self
            .channel
            .command(
                agent_arguments,
                self.plugin.clone(),
                request,
                self.ack_timeout,
            )
            .await?;
        // Frame → item mapping mirrors the CliResponse grammar the
        // connection pumps back to the plugin: Item → Ok, Error → Err
        // (non-terminal), Done/stray Ack → skipped (the stream ends
        // when the channel evicts the exchange after Done).
        Ok(Box::pin(frames.filter_map(|frame| async move {
            match frame {
                CommandFrame::Ack | CommandFrame::Done => None,
                CommandFrame::Item { item } => Some(Ok(item)),
                CommandFrame::Error { error } => {
                    Some(Err(transport_error(&error)))
                }
            }
        })))
    }
}

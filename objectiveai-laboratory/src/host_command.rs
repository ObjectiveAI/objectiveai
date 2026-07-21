//! The host's daemon-command lane: plugin MCP servers asking the
//! DAEMON to run CLI commands, over the host's own `/laboratory` WS.
//!
//! Two pieces:
//!
//! - [`CommandBridge`] — the host-wide outbound registry (moved off
//!   `HostServer` so per-session executors can hold it without an Arc
//!   cycle) plus the in-flight command exchanges. Speaks the
//!   [`HostCommandRequest`]/[`HostCommandResponse`] wire the daemon
//!   already dispatches (`websocket_laboratory.rs`), correlation ids
//!   HOST-minted — a structural mirror of the proxy's
//!   `ReverseChannel::command`/`deliver_response`.
//! - [`HostCommandExecutor`] — the SDK MCP client's
//!   [`McpClientCommandExecutor`] for plugin-laboratory connections: a
//!   `cli_request` pushed by the plugin's MCP server is forwarded to
//!   the daemon channel that owns the session and the daemon's frames
//!   stream back AS THEY ARRIVE. Regular laboratories connect with the
//!   executor's inert form (`inner: None`), which refuses execution —
//!   the container MCP there is our own injected binary and never
//!   requests commands.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use futures::{Stream, StreamExt};
use indexmap::IndexMap;
use objectiveai_sdk::cli::command::{AgentArguments, Request, ResponseItem};
use objectiveai_sdk::laboratories::daemon::{
    CommandFrame, HostCommandRequest, HostCommandResponse,
};
use objectiveai_sdk::mcp::{Error as McpError, McpClientCommandExecutor};
use tokio::sync::{mpsc, RwLock};

/// Bounds only the wait for an exchange's first frame (the daemon's
/// `Ack`); items stream with no artificial deadline (a command can be
/// a whole agent run). Mirrors the proxy's ack-timeout policy.
pub const ACK_TIMEOUT: Duration = Duration::from_secs(30);

/// One frame queued onto a channel's control lane, already framed for
/// the wire: JSON text for ordinary messages, the
/// `objectiveai_sdk::binary_frame` sandwich for chunk-bearing ones
/// (`ExportRead` replies). The writer maps these 1:1 onto WS
/// text/binary messages.
#[derive(Debug, Clone)]
pub enum LaneFrame {
    Text(String),
    Binary(Vec<u8>),
}

/// The host-wide outbound registry + in-flight command exchanges.
pub struct CommandBridge {
    /// The CONTROL lane senders, one per connected daemon channel,
    /// keyed by the host-minted registration id (moved verbatim from
    /// `HostServer::outbound` — `attach_channel`/`broadcast` reach it
    /// through here now).
    pub outbound: DashMap<u64, mpsc::UnboundedSender<LaneFrame>>,
    /// In-flight command exchanges: host-minted uuid → (owning daemon
    /// channel, frame sender). The proxy's `command_streams` shape.
    command_streams: DashMap<String, (u64, mpsc::UnboundedSender<CommandFrame>)>,
    /// In-flight Postgres tunnel streams: host-minted stream_id →
    /// (owning daemon channel, container-write sender). Daemon→host
    /// `PostgresData` bytes are routed to the container's write task
    /// through here. Same shape as `command_streams`.
    postgres_streams: DashMap<String, (u64, mpsc::UnboundedSender<Vec<u8>>)>,
}

impl CommandBridge {
    pub fn new() -> Self {
        Self {
            outbound: DashMap::new(),
            command_streams: DashMap::new(),
            postgres_streams: DashMap::new(),
        }
    }

    /// Register a Postgres tunnel stream (its container-write sender)
    /// under `stream_id`, owned by `channel`.
    pub fn register_postgres(
        &self,
        stream_id: String,
        channel: u64,
        write_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        self.postgres_streams.insert(stream_id, (channel, write_tx));
    }

    /// Route one daemon→host `PostgresData` payload to its container
    /// write task. The `channel` the frame arrived on MUST match the
    /// stream's owning channel (defends against a rogue/buggy daemon
    /// injecting into another channel's stream). Unknown / mismatched
    /// → dropped.
    pub fn deliver_postgres_data(&self, channel: u64, stream_id: &str, bytes: Vec<u8>) {
        if let Some(entry) = self.postgres_streams.get(stream_id) {
            let (owner, tx) = entry.value();
            if *owner == channel {
                let _ = tx.send(bytes);
            }
        }
    }

    /// Tear down a Postgres tunnel stream (daemon-originated close, or
    /// host-side end). Removing it drops the container-write sender,
    /// ending that stream's write task. Channel-checked like
    /// [`Self::deliver_postgres_data`].
    pub fn close_postgres(&self, channel: u64, stream_id: &str) {
        self.postgres_streams
            .remove_if(stream_id, |_, (owner, _)| *owner == channel);
    }

    /// Remove a Postgres stream unconditionally (host-side teardown:
    /// the container socket ended or the lab is evaporating).
    pub fn remove_postgres(&self, stream_id: &str) {
        self.postgres_streams.remove(stream_id);
    }

    /// Route one [`HostCommandResponse`] frame to its exchange. The
    /// parked sender survives every frame until the terminal
    /// [`CommandFrame::Done`] — or until a send fails because the
    /// consumer dropped its stream — either of which evicts the entry
    /// (the proxy's `deliver_response` contract). Unknown id →
    /// dropped.
    pub fn deliver(&self, response: HostCommandResponse) {
        let HostCommandResponse { id, frame } = response;
        let done = matches!(frame, CommandFrame::Done);
        let dead = match self.command_streams.get(&id) {
            Some(entry) => entry.value().1.send(frame).is_err(),
            None => false,
        };
        if done || dead {
            self.command_streams.remove(&id);
        }
    }

    /// Fail every in-flight exchange owned by `channel` — its daemon
    /// connection is gone, so no more frames can ever arrive. Dropping
    /// the senders ends the unfold streams (stream-end = done, the
    /// same consumer contract a proxy connection death has).
    pub fn detach(&self, channel: u64) {
        self.command_streams
            .retain(|_, (owner, _)| *owner != channel);
        // Backstop: drop this channel's Postgres streams too (the
        // per-lab cancel is the primary teardown; this catches any
        // stragglers if a channel dies without its labs evaporating).
        self.postgres_streams
            .retain(|_, (owner, _)| *owner != channel);
    }

    /// Execute one CLI command on the daemon behind `channel`,
    /// streaming the reply frames back as they arrive. Mints its OWN
    /// correlation id — NEVER one from the plugin (external untrusted
    /// code), the same rule the proxy applies.
    ///
    /// First-frame leniency, mirroring the proxy: the daemon queues
    /// `Ack` from its demux and a spawned pump emits the rest, so a
    /// pathologically fast first item could beat the Ack — ANY first
    /// frame proves the exchange live and a non-Ack one is re-prepended
    /// to the stream.
    pub async fn command(
        &self,
        channel: u64,
        agent_arguments: AgentArguments,
        plugin: objectiveai_sdk::mcp::server::Plugin,
        request: Request,
        ack_timeout: Option<Duration>,
    ) -> Result<impl Stream<Item = CommandFrame> + Send + 'static, McpError> {
        let Some(control) = self.outbound.get(&channel).map(|tx| tx.clone()) else {
            return Err(transport_error("daemon channel is not connected"));
        };
        let id = uuid::Uuid::new_v4().to_string();
        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();
        self.command_streams.insert(id.clone(), (channel, frame_tx));
        let frame = match serde_json::to_string(&HostCommandRequest {
            id: id.clone(),
            agent_arguments,
            plugin,
            request,
        }) {
            Ok(frame) => frame,
            Err(e) => {
                self.command_streams.remove(&id);
                return Err(transport_error(&format!(
                    "serialize command request: {e}"
                )));
            }
        };
        if control.send(LaneFrame::Text(frame)).is_err() {
            self.command_streams.remove(&id);
            return Err(transport_error("daemon channel closed before send"));
        }

        let first = match ack_timeout {
            Some(timeout) => {
                match tokio::time::timeout(timeout, frame_rx.recv()).await {
                    Ok(first) => first,
                    Err(_) => {
                        self.command_streams.remove(&id);
                        return Err(transport_error(
                            "daemon timed out waiting for command ack",
                        ));
                    }
                }
            }
            None => frame_rx.recv().await,
        };
        let Some(first) = first else {
            self.command_streams.remove(&id);
            return Err(transport_error("daemon channel dropped before command ack"));
        };
        let prepended = match first {
            CommandFrame::Ack => None,
            other => Some(other),
        };

        // Consumer-drop cleanup is lazy: dropping this stream drops
        // `frame_rx`, and the NEXT delivered frame's failed send
        // evicts the map entry (see [`Self::deliver`]).
        let rest = futures::stream::unfold(frame_rx, |mut rx| async move {
            rx.recv().await.map(|frame| (frame, rx))
        });
        Ok(futures::stream::iter(prepended).chain(rest))
    }
}

/// The per-session state a PLUGIN laboratory's executor carries.
pub struct PluginExecutorState {
    pub bridge: Arc<CommandBridge>,
    /// The plugin's coordinates, stamped onto every command so the
    /// daemon's run-gates apply. `mcp` is `""` — vestigial in the
    /// one-server-per-plugin world (the daemon keys on the trio).
    pub plugin: objectiveai_sdk::mcp::server::Plugin,
    /// The daemon channel that owns this session — commands go back
    /// over the SAME connection the session came in on.
    pub channel: u64,
    /// The session's LATEST request headers, full-replaced on every op
    /// (the freshest agent identity wins) and read at `execute()` time
    /// — the proxy's transient-bag semantics.
    pub transient: Arc<RwLock<IndexMap<String, String>>>,
}

/// The SDK MCP client's command executor for laboratory connections.
/// `inner: Some` on plugin-laboratory sessions; `None` everywhere else
/// (execution refused — regular lab containers never request it).
#[derive(Clone)]
pub struct HostCommandExecutor {
    pub inner: Option<Arc<PluginExecutorState>>,
}

impl McpClientCommandExecutor for HostCommandExecutor {
    type Error = McpError;
    type Stream = Pin<Box<dyn Stream<Item = Result<ResponseItem, McpError>> + Send>>;

    async fn execute(&self, request: Request) -> Result<Self::Stream, Self::Error> {
        let Some(state) = self.inner.as_ref() else {
            return Err(transport_error(
                "command execution is not supported on this laboratory",
            ));
        };
        let agent_arguments =
            AgentArguments::from_transient_headers(&*state.transient.read().await);
        let frames = state
            .bridge
            .command(
                state.channel,
                agent_arguments,
                state.plugin.clone(),
                request,
                Some(ACK_TIMEOUT),
            )
            .await?;
        // Frame → item mapping mirrors the CliResponse grammar the
        // connection pumps back to the plugin: Item → Ok, Error → Err
        // (non-terminal), Done/stray Ack → skipped (the stream ends
        // when the bridge evicts the exchange after Done).
        Ok(Box::pin(frames.filter_map(|frame| async move {
            match frame {
                CommandFrame::Ack | CommandFrame::Done => None,
                CommandFrame::Item { item } => Some(Ok(item)),
                CommandFrame::Error { error } => Some(Err(transport_error(&error))),
            }
        })))
    }
}

/// The same wire-level error shape the proxy's reverse channel uses.
fn transport_error(message: &str) -> McpError {
    McpError::MalformedResponse {
        url: "ws".to_string(),
        message: message.to_string(),
    }
}

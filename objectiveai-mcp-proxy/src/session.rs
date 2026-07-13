//! Per-session state and per-session dispatch.
//!
//! A `Session` owns the upstream MCP connections that belong to one MCP
//! session and is responsible for fanning `tools/list` / `resources/list`
//! out to them and routing `tools/call` / `resources/read` to the right
//! upstream. The registry that minds session ids and hands out
//! `Arc<Session>`s lives in [`crate::session_manager`].

use dashmap::DashMap;
use futures::future::try_join_all;
use indexmap::IndexMap;
use std::sync::Arc;
use objectiveai_sdk::laboratories::{ClientLaboratory, Laboratory};
use objectiveai_sdk::client_objectiveai_mcp::server_request;
use objectiveai_sdk::mcp::{
    JsonRpcNotification,
    resource::{ListResourcesResult, ReadResourceResult, Resource},
    server::{ListServersResult, Server},
    tool::{
        CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult,
        TextContent, Tool, ToolSchemaObject, ToolSchemaType,
    },
};

use crate::reverse_channel::Upstream;

/// Name of the proxy-native tool injected when a session has 2+
/// laboratories. Reserved — intercepted by exact match in
/// [`Session::call_tool`] before prefix routing, so it never forwards to
/// an upstream.
pub const LABORATORY_TRANSFER_TOOL: &str = "laboratory_transfer";
use axum::http::HeaderMap;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;

/// Capacity of the per-session outbound notification channel. Sized so
/// even a noisy upstream can't easily lap a slow SSE consumer.
const OUTBOUND_CAPACITY: usize = 64;

/// Hashable key for a JSON-RPC request id. JSON-RPC ids can be number,
/// string, or null, so we serialize to canonical JSON and hash on that.
fn request_id_key(id: &serde_json::Value) -> String {
    // serde_json::to_string is infallible for any Value.
    serde_json::to_string(id).unwrap_or_default()
}

/// Per-session state.
///
/// All routing, fan-out, and forwarding methods live here. The registry
/// that hands out `Arc<Session>`s by id is `SessionManager` (in
/// [`crate::session_manager`]).
#[derive(Debug)]
pub struct Session {
    /// Live upstream MCP connections keyed by their **routing prefix** — a
    /// `_`- and `.`-free token derived from the upstream's
    /// `server_info.name` (with `-{version}` / `-{index}` escalation on
    /// collision; see [`crate::session_manager::build_prefix_map`]). The key
    /// is the same string the proxy uses as the `<prefix>_` prefix on every
    /// tool name and resource URI it ships, so routing inbound `tools/call`
    /// / `resources/read` is a split-on-first-`_` plus direct map lookup —
    /// no longest-prefix scan, no side-channel cache to keep coherent.
    ///
    /// Built once at construction and never mutated. Insertion order is
    /// url-sorted (so fresh and resumed sessions agree on prefixes);
    /// `list_tools` / `list_resources` sort their output by name/URI
    /// independently, so insertion order doesn't affect listings.
    ///
    /// `Connection` is itself a cheaply-clonable Arc wrapper; dropping a
    /// `Connection` fires the upstream listener's wakeup signal so it can
    /// self-cancel within scheduler latency once no external handle remains.
    pub connections: IndexMap<String, Upstream>,
    /// Fan-out channel for server-initiated notifications. Whenever an
    /// upstream emits `notifications/tools/list_changed` or
    /// `notifications/resources/list_changed`, a JsonRpcNotification with
    /// the matching method is published here. Subscribers (the SSE GET
    /// stream in `mcp::handle_get`) drain it onto the wire to the
    /// downstream client.
    ///
    /// `broadcast` rather than `mpsc` so multiple concurrent GET streams
    /// for the same session — which the MCP spec allows — each see every
    /// notification.
    pub outbound: broadcast::Sender<JsonRpcNotification>,
    /// In-flight per-request cancellation tokens, keyed by the inbound
    /// JSON-RPC request id (stringified for hashability — JSON-RPC ids
    /// can be number, string, or null). The downstream client cancels a
    /// request by sending `notifications/cancelled` with the matching
    /// `requestId`; the handler that owns that id observes the token
    /// firing via `tokio::select!` and returns a `-32800 request cancelled`
    /// JSON-RPC error. Drops the upstream call's future as a side effect.
    in_flight: DashMap<String, CancellationToken>,
    /// Session-global header overrides stamped on every outbound
    /// request to every upstream. The only keys ever recorded are the
    /// agent-identity + response-routing headers (see
    /// [`Self::TRANSIENT_HEADER_KEYS`]) — extracted from inbound
    /// `initialize` HeaderMaps by [`Self::apply_transient_headers`].
    ///
    /// Reads dominate writes: every outbound request through any
    /// upstream Connection reads (via [`Connection::set_extra_headers`]
    /// applied on the Connection's own RwLock); writes fire only on
    /// inbound `initialize` (reuse or fresh connect). `RwLock` matches
    /// the read/write ratio.
    pub transient_headers: RwLock<IndexMap<String, String>>,
}

impl Session {
    pub(crate) fn new(
        connections: IndexMap<String, Upstream>,
    ) -> Self {
        let (outbound, _) = broadcast::channel(OUTBOUND_CAPACITY);

        // Wire each upstream's list_changed callbacks to publish a
        // matching notification onto the outbound channel. Callbacks fire
        // under the upstream's cache write lock (before the network
        // refresh), so by the time the downstream client re-fetches via
        // tools/list or resources/list it'll either get the new list or
        // wait on the lock until the new list lands — never sees the
        // stale list with a fresh notification.
        for connection in connections.values() {
            let tx = outbound.clone();
            connection.set_on_tools_list_changed(move || {
                let _ = tx.send(JsonRpcNotification {
                    jsonrpc: "2.0".into(),
                    method: "notifications/tools/list_changed".into(),
                    params: None,
                });
            });
            let tx = outbound.clone();
            connection.set_on_resources_list_changed(move || {
                let _ = tx.send(JsonRpcNotification {
                    jsonrpc: "2.0".into(),
                    method: "notifications/resources/list_changed".into(),
                    params: None,
                });
            });
        }

        Self {
            connections,
            outbound,
            in_flight: DashMap::new(),
            transient_headers: RwLock::new(IndexMap::new()),
        }
    }

    /// Header keys persisted on the session-global transient bag —
    /// extracted from inbound `initialize` headers and stamped on
    /// every outbound upstream request via
    /// [`Connection::set_extra_headers`]. Closed set; not extensible.
    ///
    /// Each `initialize` re-extracts these from the inbound `HeaderMap`
    /// and full-replaces the bag (missing keys drop).
    pub const TRANSIENT_HEADER_KEYS: [&'static str; 6] = [
        "X-OBJECTIVEAI-RESPONSE-ID",
        "X-OBJECTIVEAI-RESPONSE-IDS",
        "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY",
        "X-OBJECTIVEAI-AGENT-ID",
        "X-OBJECTIVEAI-AGENT-FULL-ID",
        "X-OBJECTIVEAI-AGENT-REMOTE",
    ];

    /// Build the transient bag from an inbound `initialize` HeaderMap,
    /// FULL-REPLACE [`Self::transient_headers`] with it, then fan the
    /// new bag onto every upstream `Connection`'s `extra_headers` via
    /// [`Connection::set_extra_headers`].
    ///
    /// Missing keys in `src` → absent from the new bag → dropped from
    /// every Connection's `extra_headers` too. Never merges with the
    /// previous bag; this is the "replace, even if missing some" rule.
    pub async fn apply_transient_headers(&self, src: &HeaderMap) {
        let mut bag = IndexMap::new();
        for key in Self::TRANSIENT_HEADER_KEYS {
            if let Some(v) = src.get(key).and_then(|v| v.to_str().ok()) {
                bag.insert(key.to_string(), v.to_string());
            }
        }
        let snapshot = bag.clone();
        *self.transient_headers.write().await = bag;
        for connection in self.connections.values() {
            connection.set_extra_headers(snapshot.clone()).await;
        }
    }

    /// Mint a [`CancellationToken`] for an inbound request id, store it,
    /// and hand back a clone. The handler `select!`s on the clone; the
    /// stored token is what `cancel_in_flight` fires.
    pub fn register_in_flight(&self, id: &serde_json::Value) -> CancellationToken {
        let token = CancellationToken::new();
        self.in_flight.insert(request_id_key(id), token.clone());
        token
    }

    /// Drop the in-flight token for `id`. Always paired with an earlier
    /// `register_in_flight` via a guard so we don't leak entries on the
    /// happy path.
    pub fn deregister_in_flight(&self, id: &serde_json::Value) {
        self.in_flight.remove(&request_id_key(id));
    }

    /// Fire the cancellation token associated with `id`, if any. Returns
    /// `true` if a token was found and cancelled. Triggered by an inbound
    /// `notifications/cancelled` from the downstream client.
    pub fn cancel_in_flight(&self, id: &serde_json::Value) -> bool {
        match self.in_flight.get(&request_id_key(id)) {
            Some(entry) => {
                entry.value().cancel();
                true
            }
            None => false,
        }
    }

    /// Fan `tools/list` out to every upstream in parallel, prefix each
    /// tool's name with `<server-name>_`, concatenate the per-upstream
    /// lists, and return the union sorted by name. Fails fast: the
    /// first upstream error short-circuits via `try_join_all` and is
    /// returned to the caller — we don't paper over a broken upstream.
    ///
    /// Sorting by name guarantees a stable order across calls regardless
    /// of upstream `HashMap` iteration order or per-upstream return
    /// order; downstream consumers (e.g. seeded mock agents) rely on
    /// this for deterministic output.
    pub async fn list_tools(&self) -> Result<ListToolsResult, Arc<objectiveai_sdk::mcp::Error>> {
        self.list_tools_filtered(None, None).await
    }

    /// Per-upstream variant of [`Self::list_tools`]. Each provided filter
    /// narrows the set of upstreams that participate (ANDed together):
    /// `filter_url` keeps only the upstream whose connection `url` matches
    /// verbatim; `filter_name` keeps only the upstream whose routing
    /// prefix (the name `servers/list` reports) matches. `None` for both
    /// behaves like the no-arg form (fan out to every upstream).
    ///
    /// An unmatched filter returns an empty `ListToolsResult` (not an
    /// error) — the caller validates whether emptiness is acceptable. The
    /// per-upstream allowlist (`X-MCP-Tools-Allow`) is still applied to
    /// whichever upstream(s) participate.
    pub async fn list_tools_filtered(
        &self,
        filter_url: Option<&str>,
        filter_name: Option<&str>,
    ) -> Result<ListToolsResult, Arc<objectiveai_sdk::mcp::Error>> {
        let pairs: Vec<(&String, &Upstream)> = self
            .connections
            .iter()
            .filter(|(name, c)| {
                filter_url.is_none_or(|u| c.url() == u)
                    && filter_name.is_none_or(|n| name.as_str() == n)
            })
            .collect();
        let results = try_join_all(
            pairs
                .iter()
                .map(|(_, c)| async move {
                    let r = c.list_tools().await;
                    r
                }),
        )
        .await?;

        let mut tools: Vec<Tool> = Vec::new();
        for ((server_name, _), arc) in pairs.into_iter().zip(results) {
            for tool in arc.iter() {
                let mut prefixed = tool.clone();
                prefixed.name = prefix_name(server_name, &tool.name);
                tools.push(prefixed);
            }
        }

        // Proxy-native `laboratory_transfer` — visible only when listing the
        // full surface (not a per-server filtered view) and the session has
        // 2+ laboratories to move files between.
        if filter_url.is_none()
            && filter_name.is_none()
            && self.laboratory_count() >= 2
        {
            tools.push(laboratory_transfer_tool());
        }

        tools.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            _meta: None,
        })
    }

    /// List every connected upstream MCP server with its metadata. Unlike
    /// `list_tools`/`list_resources` this is a proxy-local aggregate: each
    /// upstream's `initialize` reply is already held in memory, so there's
    /// no fan-out / network call. One [`Server`] per connection, keyed by
    /// the routing prefix (the same `<prefix>_` prepended to that server's
    /// tools/resources), sorted by name for stable output.
    pub fn list_servers(&self) -> ListServersResult {
        let mut servers: Vec<Server> = self
            .connections
            .iter()
            .map(|(prefix, up)| {
                let laboratory = up.laboratory();
                // The assistant-facing composite id — what
                // `laboratory_transfer` takes. `None` for markers
                // predating machine tracking (nothing to compose).
                let laboratory_id = laboratory.as_ref().and_then(|lab| {
                    let Laboratory::Client(c) = lab;
                    c.composite_id()
                });
                Server {
                    name: prefix.clone(),
                    url: up.url().to_string(),
                    initialize_result: up.initialize_result().clone(),
                    laboratory,
                    laboratory_id,
                    plugin: up.plugin(),
                }
            })
            .collect();
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        ListServersResult { servers }
    }

    /// Fan `resources/list` out to every upstream in parallel, prefix
    /// each URI with `<server-name>_`, concatenate the per-upstream
    /// lists, and return the union sorted by URI. Same fail-fast
    /// semantics as [`Session::list_tools`] — the first upstream error
    /// short-circuits and is returned to the caller.
    pub async fn list_resources(&self) -> Result<ListResourcesResult, Arc<objectiveai_sdk::mcp::Error>> {
        self.list_resources_filtered(None, None).await
    }

    /// Per-upstream variant of [`Self::list_resources`]. Each provided
    /// filter narrows the participating upstreams (ANDed): `filter_url`
    /// matches the connection `url`; `filter_name` matches the routing
    /// prefix (the name `servers/list` reports). `None` for both fans out
    /// to every upstream.
    ///
    /// An unmatched filter returns an empty `ListResourcesResult` (not an
    /// error) — the caller validates whether emptiness is acceptable.
    pub async fn list_resources_filtered(
        &self,
        filter_url: Option<&str>,
        filter_name: Option<&str>,
    ) -> Result<ListResourcesResult, Arc<objectiveai_sdk::mcp::Error>> {
        let pairs: Vec<(&String, &Upstream)> = self
            .connections
            .iter()
            .filter(|(name, c)| {
                filter_url.is_none_or(|u| c.url() == u)
                    && filter_name.is_none_or(|n| name.as_str() == n)
            })
            .collect();
        let results = try_join_all(
            pairs
                .iter()
                .map(|(_, c)| async move {
                    let r = c.list_resources().await;
                    r
                }),
        )
        .await?;

        let mut resources: Vec<Resource> = Vec::new();
        for ((server_name, _), arc) in pairs.into_iter().zip(results) {
            for resource in arc.iter() {
                let mut prefixed = resource.clone();
                prefixed.uri = prefix_name(server_name, &resource.uri);
                resources.push(prefixed);
            }
        }
        resources.sort_by(|a, b| a.uri.cmp(&b.uri));

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            _meta: None,
        })
    }

    /// Forward `tools/call` to whichever upstream owns the named tool.
    /// Routing is longest-prefix-match against the connection map's keys —
    /// see [`Session::route`].
    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, CallToolError> {
        // Proxy-native tool — intercept by exact name BEFORE prefix routing
        // so it can't be shadowed by (or mistaken for) a `<prefix>_<tool>`.
        if params.name == LABORATORY_TRANSFER_TOOL {
            return Ok(self.laboratory_transfer(params).await);
        }

        let (connection, original_name) = self
            .route(&params.name)
            .ok_or_else(|| CallToolError::ToolNotFound(params.name.clone()))?;

        // Forward to the upstream with the un-prefixed tool name it actually
        // knows; pass everything else (`arguments`, `task`, `_meta`) through
        // unchanged.
        let upstream_params = CallToolRequestParams {
            name: original_name,
            arguments: params.arguments.clone(),
            task: params.task.clone(),
            _meta: params._meta.clone(),
        };
        let r = connection.call_tool(&upstream_params).await;
        Ok(r?)
    }

    /// Number of laboratory upstreams in this session (typed marker, not
    /// URL-derived). Drives the `laboratory_transfer` tool's visibility.
    fn laboratory_count(&self) -> usize {
        self.connections
            .values()
            .filter(|u| u.laboratory().is_some())
            .count()
    }

    /// Find the upstream that IS the laboratory `target` — matched on
    /// the FULL identity (id + machine + machine_state; laboratory ids
    /// are only unique per (machine, state)), never a routing prefix,
    /// a name, or a bare id. Returns the upstream and its typed
    /// marker, whose (machine, machine_state) pair pins the exact
    /// host for downstream routing.
    fn find_laboratory(&self, target: &ClientLaboratory) -> Option<(&Upstream, Laboratory)> {
        self.connections.values().find_map(|u| match u.laboratory() {
            Some(Laboratory::Client(c)) if c == *target => {
                Some((u, Laboratory::Client(c)))
            }
            _ => None,
        })
    }

    /// Handle the proxy-native `laboratory_transfer` tool: copy a
    /// file/folder from one laboratory to another. Source/destination
    /// are the COMPOSITE `{machineID}/{state}/{laboratoryID}` full
    /// identity (the `laboratory_id` from `agents mcp servers list`),
    /// REQUIRED: laboratory ids are only unique per (machine, state),
    /// so a bare id is ambiguous and rejected. Resolved against the
    /// typed markers on all three fields.
    /// The byte movement is DELEGATED: the proxy matches the two
    /// laboratories' enum variants and, for client↔client, sends ONE
    /// combined request down the reverse channel — `local_transfer`
    /// when both share a (machine, state) pair (the one laboratory
    /// host pipes container-to-container), plain `transfer` otherwise
    /// (the CLI daemon drives the export/import splice, one chunk in
    /// transit). The proxy never touches payload bytes. Future
    /// `Laboratory::Server` variants extend the match with their own
    /// (more involved) orchestration.
    async fn laboratory_transfer(&self, params: &CallToolRequestParams) -> CallToolResult {
        let arg = |key: &str| -> Option<&str> {
            params
                .arguments
                .as_ref()
                .and_then(|m| m.get(key))
                .and_then(|v| v.as_str())
        };
        let (source, source_path, destination, destination_path) = match (
            arg("source"),
            arg("source_path"),
            arg("destination"),
            arg("destination_path"),
        ) {
            (Some(s), Some(sp), Some(d), Some(dp)) => (s, sp, d, dp),
            _ => {
                return transfer_error(
                    "laboratory_transfer requires string arguments: source, \
                     source_path, destination, destination_path",
                );
            }
        };
        // The composite is REQUIRED: `{machineID}/{state}/{labID}`
        // with the state and laboratory id base62-encoded — the full
        // identity, since laboratory ids are only unique per
        // (machine, state). Malformed args (a bare id included) fail
        // here with the expected shape.
        let composite_error = |which: &str, value: &str| {
            transfer_error(format!(
                "{which} '{value}' is not a composite laboratory id — expected \
                 `{{machineID}}/{{state}}/{{laboratoryID}}` (state and laboratory id \
                 base62-encoded), exactly as `laboratory_id` from \
                 `agents mcp servers list` reports it"
            ))
        };
        let Some(source_target) = ClientLaboratory::from_composite_id(source) else {
            return composite_error("source", source);
        };
        let Some(dest_target) = ClientLaboratory::from_composite_id(destination) else {
            return composite_error("destination", destination);
        };
        // Compare the PARSED identities (canonical), not the raw arg
        // strings.
        if source_target == dest_target {
            return transfer_error("source and destination must be different laboratories");
        }
        let (source_channel, source_marker) = match self.find_laboratory(&source_target) {
            Some((u, marker)) => match u.reverse_channel() {
                Some(c) => (c, marker),
                None => return transfer_error("source laboratory has no reverse channel"),
            },
            None => {
                return transfer_error(format!("no laboratory with id '{source}' in this session"));
            }
        };
        let (dest_channel, dest_marker) = match self.find_laboratory(&dest_target) {
            Some((u, marker)) => match u.reverse_channel() {
                Some(c) => (c, marker),
                None => return transfer_error("destination laboratory has no reverse channel"),
            },
            None => {
                return transfer_error(format!(
                    "no laboratory with id '{destination}' in this session"
                ));
            }
        };

        // Dispatch by the two markers' enum variants. Client↔client
        // collapses to ONE combined request on the SOURCE upstream's
        // reverse channel (both channels reach the same client today;
        // source is the documented pick). Adding a Laboratory::Server
        // variant makes this match non-exhaustive — its arms carry
        // their own orchestration, they are NOT a forwarded request.
        let bytes = match (source_marker, dest_marker) {
            (Laboratory::Client(src), Laboratory::Client(dst)) => {
                let local = src.machine.is_some()
                    && src.machine == dst.machine
                    && src.machine_state.is_some()
                    && src.machine_state == dst.machine_state;
                if local {
                    let request = server_request::LaboratoryLocalTransferRequest {
                        source_id: src.id,
                        source_machine: src.machine,
                        source_machine_state: src.machine_state,
                        source_path: source_path.to_string(),
                        destination_id: dst.id,
                        destination_machine: dst.machine,
                        destination_machine_state: dst.machine_state,
                        destination_path: destination_path.to_string(),
                    };
                    match source_channel.laboratory_local_transfer(request).await {
                        Ok(bytes) => bytes,
                        Err(e) => return transfer_error(format!("transfer failed: {e}")),
                    }
                } else {
                    let request = server_request::LaboratoryTransferRequest {
                        source_id: src.id,
                        source_machine: src.machine,
                        source_machine_state: src.machine_state,
                        source_path: source_path.to_string(),
                        destination_id: dst.id,
                        destination_machine: dst.machine,
                        destination_machine_state: dst.machine_state,
                        destination_path: destination_path.to_string(),
                    };
                    match source_channel.laboratory_transfer(request).await {
                        Ok(bytes) => bytes,
                        Err(e) => return transfer_error(format!("transfer failed: {e}")),
                    }
                }
            }
        };
        transfer_text(format!(
            "transferred {bytes} bytes: {source}:{source_path} → {destination}:{destination_path}",
        ))
    }

    /// Forward `resources/read` to whichever upstream owns the URI. Same
    /// longest-prefix-match routing as [`Session::call_tool`].
    pub async fn read_resource(
        &self,
        uri: &str,
    ) -> Result<ReadResourceResult, ReadResourceError> {
        let (connection, original_uri) = self
            .route(uri)
            .ok_or_else(|| ReadResourceError::ResourceNotFound(uri.to_string()))?;
        let r = connection.read_resource(&original_uri).await;
        Ok(r?)
    }

    /// Resolve a `<prefix>_<original>` identifier to the owning connection
    /// and the original (un-prefixed) name the upstream actually knows.
    ///
    /// Routing prefixes are built `_`-free (see
    /// [`crate::session_manager::build_prefix_map`]), so the **first** `_`
    /// is always the prefix/original boundary — split once, look the prefix
    /// up in the fixed map. The original part is forwarded verbatim; it is
    /// NOT checked against the upstream's tool/resource list, so resource
    /// templates and otherwise-unlisted names still pass through. A missing
    /// `_`, or a prefix not in the map, yields `None`.
    fn route<'a>(&'a self, prefixed: &str) -> Option<(&'a Upstream, String)> {
        let (prefix, rest) = prefixed.split_once('_')?;
        let connection = self.connections.get(prefix)?;
        Some((connection, rest.to_string()))
    }
}

/// Prefix a tool name or resource URI with the upstream server name.
/// Format: `<server-name>_<original>`.
fn prefix_name(server_name: &str, name: &str) -> String {
    format!("{server_name}_{name}")
}

/// The proxy-native `laboratory_transfer` tool definition, injected into
/// `tools/list` when a session has 2+ laboratories. Source/destination are
/// COMPOSITE laboratory ids `{machineID}/{state}/{laboratoryID}` — the
/// `laboratory_id` from `agents mcp servers list`, also stated in each
/// laboratory server's instructions. Quoted verbatim, never constructed
/// by hand (the state and laboratory id segments are base62-encoded).
fn laboratory_transfer_tool() -> Tool {
    fn string_prop(description: &str) -> serde_json::Value {
        serde_json::json!({ "type": "string", "description": description })
    }
    let mut properties: IndexMap<String, serde_json::Value> = IndexMap::new();
    properties.insert(
        "source".to_string(),
        string_prop(
            "The source laboratory's full id — the `laboratory_id` from \
             `agents mcp servers list` (`{machine}/{state}/{id}`), quoted \
             verbatim.",
        ),
    );
    properties.insert(
        "source_path".to_string(),
        string_prop("Absolute path of the file or folder to copy from the source laboratory."),
    );
    properties.insert(
        "destination".to_string(),
        string_prop(
            "The destination laboratory's full id — same form as `source`, \
             quoted verbatim.",
        ),
    );
    properties.insert(
        "destination_path".to_string(),
        string_prop(
            "Absolute destination directory in the destination laboratory; the \
             source's basename is created inside it.",
        ),
    );
    Tool {
        name: LABORATORY_TRANSFER_TOOL.to_string(),
        title: Some("Laboratory Transfer".to_string()),
        description: Some(
            "Copy a file or folder from one laboratory to another (streamed). \
             Identify the laboratories by their FULL `laboratory_id` from \
             `agents mcp servers list` (`{machine}/{state}/{id}` — also stated \
             in each laboratory server's instructions), quoted verbatim."
                .to_string(),
        ),
        icons: None,
        input_schema: ToolSchemaObject {
            r#type: ToolSchemaType::Object,
            properties: Some(properties),
            required: Some(vec![
                "source".to_string(),
                "source_path".to_string(),
                "destination".to_string(),
                "destination_path".to_string(),
            ]),
            extra: IndexMap::new(),
        },
        output_schema: None,
        annotations: None,
        execution: None,
        _meta: None,
    }
}

/// Build a successful `laboratory_transfer` `CallToolResult` (text).
fn transfer_text(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text,
            annotations: None,
            _meta: None,
        })],
        structured_content: None,
        is_error: None,
        _meta: None,
    }
}

/// Build an error `laboratory_transfer` `CallToolResult` (`isError: true`)
/// so the failure surfaces to the agent as a tool error, not a protocol
/// error.
fn transfer_error(text: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: text.into(),
            annotations: None,
            _meta: None,
        })],
        structured_content: None,
        is_error: Some(true),
        _meta: None,
    }
}

/// Failure modes for [`Session::call_tool`].
#[derive(Debug, thiserror::Error)]
pub enum CallToolError {
    #[error("tool not found on any upstream: {0}")]
    ToolNotFound(String),
    #[error("upstream call_tool failed: {0}")]
    Upstream(#[from] objectiveai_sdk::mcp::Error),
}

/// Failure modes for [`Session::read_resource`].
#[derive(Debug, thiserror::Error)]
pub enum ReadResourceError {
    #[error("resource not found on any upstream: {0}")]
    ResourceNotFound(String),
    #[error("upstream read_resource failed: {0}")]
    Upstream(#[from] objectiveai_sdk::mcp::Error),
}


//! Shared test rig for the proxy compliance suite.
//!
//! Spawns the proxy and N test upstreams as subprocesses, waits for
//! them to bind, hands the test an rmcp `Client` connected through the
//! proxy with whatever custom session-init headers the test asks for.
//!
//! The proxy binary is located via `CARGO_BIN_EXE_objectiveai-mcp-proxy`
//! (cargo sets this for tests in the same package). The test upstream
//! binary is computed from `CARGO_MANIFEST_DIR` — you must run
//! `cargo build --workspace` (or equivalently
//! `cargo build -p test-upstream`) before `cargo test`, or use
//! `objectiveai-mcp-proxy/test.sh` which handles both.

#![allow(dead_code)] // Each integration test only uses a subset of helpers.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use test_upstream::{TestResource, TestTool};
use rmcp::ServiceExt;
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde::Serialize;
use tokio::process::{Child, Command};
use tokio::time::sleep;

const READY_TIMEOUT: Duration = Duration::from_secs(15);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Description of one test upstream the rig should spawn.
#[derive(Debug, Clone)]
pub struct UpstreamSpec {
    pub server_name: String,
    pub initial_tools: Vec<TestTool>,
    pub initial_resources: Vec<TestResource>,
    pub require_auth: Option<String>,
    pub header_gate: Option<(String, String)>,
}

impl UpstreamSpec {
    pub fn new(server_name: impl Into<String>) -> Self {
        Self {
            server_name: server_name.into(),
            initial_tools: Vec::new(),
            initial_resources: Vec::new(),
            require_auth: None,
            header_gate: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<TestTool>) -> Self {
        self.initial_tools = tools;
        self
    }

    pub fn with_resources(mut self, resources: Vec<TestResource>) -> Self {
        self.initial_resources = resources;
        self
    }

    pub fn with_require_auth(mut self, value: impl Into<String>) -> Self {
        self.require_auth = Some(value.into());
        self
    }

    pub fn with_header_gate(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.header_gate = Some((name.into(), value.into()));
        self
    }
}

/// Handle to a spawned upstream subprocess.
pub struct Upstream {
    pub url: String,
    pub control_base: String,
    child: Child,
}

impl Drop for Upstream {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Handle to the spawned proxy subprocess.
pub struct Proxy {
    pub url: String,
    child: Child,
}

impl Drop for Proxy {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Full test rig: proxy + upstreams. Drop to tear everything down.
pub struct TestRig {
    pub proxy: Proxy,
    pub upstreams: Vec<Upstream>,
}

impl TestRig {
    /// Spawn the proxy and one test upstream per spec, in parallel.
    pub async fn new(specs: Vec<UpstreamSpec>) -> Self {
        let mut upstreams = Vec::with_capacity(specs.len());
        for spec in specs {
            upstreams.push(spawn_upstream(spec).await);
        }
        let proxy = spawn_proxy().await;
        Self { proxy, upstreams }
    }

    /// Convenience: render the upstream URLs as a JSON array suitable
    /// for the `X-MCP-Servers` header.
    pub fn x_mcp_servers(&self) -> String {
        let urls: Vec<&str> = self.upstreams.iter().map(|u| u.url.as_str()).collect();
        serde_json::to_string(&urls).unwrap()
    }

    /// Connect a fresh rmcp client through the proxy with the given
    /// custom session-init headers (`X-MCP-Servers` etc).
    pub async fn connect_client(
        &self,
        custom_headers: HashMap<&str, String>,
    ) -> RunningService<rmcp::RoleClient, rmcp::model::ClientInfo> {
        use reqwest::header::{HeaderName, HeaderValue};

        let mut headers: HashMap<HeaderName, HeaderValue> = HashMap::new();
        for (k, v) in custom_headers {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes()).expect("valid header name"),
                HeaderValue::from_str(&v).expect("valid header value"),
            );
        }

        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(self.proxy.url.clone())
                .custom_headers(headers),
        );

        client_info_for_proxy()
            .serve(transport)
            .await
            .expect("client serve")
    }

    /// Hit a test upstream's `POST /__test/set-tools` endpoint to swap
    /// its tool list. The upstream broadcasts
    /// `notifications/tools/list_changed` to its session, which the
    /// proxy then forwards to the downstream client.
    pub async fn set_upstream_tools(&self, idx: usize, tools: Vec<TestTool>) {
        #[derive(Serialize)]
        struct Body {
            tools: Vec<TestTool>,
        }
        let url = format!("{}/set-tools", self.upstreams[idx].control_base);
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&Body { tools })
            .send()
            .await
            .expect("control POST");
        assert!(resp.status().is_success(), "set-tools failed: {}", resp.status());
    }

    /// Hit a test upstream's `POST /__test/set-resources` endpoint.
    pub async fn set_upstream_resources(&self, idx: usize, resources: Vec<TestResource>) {
        #[derive(Serialize)]
        struct Body {
            resources: Vec<TestResource>,
        }
        let url = format!("{}/set-resources", self.upstreams[idx].control_base);
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&Body { resources })
            .send()
            .await
            .expect("control POST");
        assert!(resp.status().is_success(), "set-resources failed: {}", resp.status());
    }

    /// Read the headers the most-recent MCP POST to upstream `idx` carried.
    pub async fn upstream_seen_headers(&self, idx: usize) -> HashMap<String, String> {
        let url = format!("{}/seen-headers", self.upstreams[idx].control_base);
        reqwest::get(&url)
            .await
            .expect("control GET")
            .json()
            .await
            .expect("control JSON")
    }
}

// ---- Subprocess spawn helpers ------------------------------------------

async fn spawn_upstream(spec: UpstreamSpec) -> Upstream {
    let port = pick_free_port();
    let mut cmd = Command::new(test_upstream_binary());
    cmd.env("ADDRESS", "127.0.0.1")
        .env("PORT", port.to_string())
        .env("SERVER_NAME", &spec.server_name)
        .env(
            "INITIAL_TOOLS_JSON",
            serde_json::to_string(&spec.initial_tools).unwrap(),
        )
        .env(
            "INITIAL_RESOURCES_JSON",
            serde_json::to_string(&spec.initial_resources).unwrap(),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);

    if let Some(auth) = spec.require_auth {
        cmd.env("REQUIRE_AUTH", auth);
    }
    if let Some((name, value)) = spec.header_gate {
        cmd.env("HEADER_GATE_NAME", name).env("HEADER_GATE_VALUE", value);
    }

    let child = cmd.spawn().expect("spawn test-upstream");
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    wait_for_listening(addr).await;

    Upstream {
        url: format!("http://127.0.0.1:{port}/"),
        control_base: format!("http://127.0.0.1:{port}/__test"),
        child,
    }
}

async fn spawn_proxy() -> Proxy {
    let port = pick_free_port();
    let child = Command::new(env!("CARGO_BIN_EXE_objectiveai-mcp-proxy"))
        .env("ADDRESS", "127.0.0.1")
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn proxy");
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    wait_for_listening(addr).await;
    Proxy {
        url: format!("http://127.0.0.1:{port}/"),
        child,
    }
}

fn pick_free_port() -> u16 {
    // Bind to 0, read the assigned port, drop the listener. Tiny TOCTOU
    // window before the spawned binary re-binds; acceptable for tests.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

async fn wait_for_listening(addr: SocketAddr) {
    let started = Instant::now();
    loop {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        if started.elapsed() > READY_TIMEOUT {
            panic!("port {addr} did not start listening within {READY_TIMEOUT:?}");
        }
        sleep(POLL_INTERVAL).await;
    }
}

/// Build a [`ClientInfo`](rmcp::model::ClientInfo) whose `protocol_version`
/// is `"2025-06-18"` — the version the proxy is pinned to. rmcp 0.16's
/// `ProtocolVersion::LATEST` is `"2025-03-26"`, which the proxy rejects.
/// We round-trip through serde because `ProtocolVersion`'s inner field is
/// private — the deserializer falls through to `Cow::Owned(s)` for any
/// unknown string.
fn client_info_for_proxy() -> rmcp::model::ClientInfo {
    let value = serde_json::json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {
            "name": "objectiveai-mcp-proxy-test",
            "version": env!("CARGO_PKG_VERSION"),
        }
    });
    serde_json::from_value(value).expect("ClientInfo deserialize")
}

fn test_upstream_binary() -> PathBuf {
    // CARGO_MANIFEST_DIR is the proxy crate root (...../objectiveai-mcp-proxy).
    // Workspace target/ sits one level up.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();

    // Always run `cargo build -p test-upstream` first so the binary on
    // disk reflects the current source. Cargo's incremental check is
    // fast when the source is unchanged. Previously this just returned
    // the first existing path under target/{debug,release}, which
    // silently ran tests against a stale binary if the source had
    // changed since the last build.
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = std::process::Command::new(&cargo)
        .args(["build", "-p", "test-upstream"])
        .status()
        .expect("failed to spawn cargo build");
    assert!(status.success(), "cargo build -p test-upstream failed");

    let bin_name = if cfg!(windows) {
        "test-upstream.exe"
    } else {
        "test-upstream"
    };
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let candidate = target.join("debug").join(bin_name);
    assert!(candidate.exists(), "test-upstream binary missing at {candidate:?}");
    candidate
}

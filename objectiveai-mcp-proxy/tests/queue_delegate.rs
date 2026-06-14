//! In-process [`QueueDelegate`] splice tests — the replacement for
//! the retired HTTP `/notify` surface (#218).
//!
//! The proxy no longer owns a notification queue: the embedder (the
//! API) installs a delegate at `setup()` time, and after each
//! SUCCESSFUL `tools/call` the proxy asks it for pending content
//! blocks, splicing whatever comes back ahead of the tool output —
//! wrapped in the SDK-owned `<system-reminder>` text-block pair
//! whose prefix carries the delegate's confirmation token, plus a
//! `_meta.notifications` block count.
//!
//! These tests run the proxy IN-PROCESS (the subprocess rig cannot
//! inject a delegate) against a real spawned test upstream, driving
//! the same raw-HTTP session/tool-call helpers the rest of the
//! suite uses.

mod common;
mod notify_helpers;

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use common::{Upstream, UpstreamSpec, spawn_upstream};
use indexmap::IndexMap;
use notify_helpers::{call_tool, content_blocks, init_session};
use objectiveai_mcp_proxy::{Config, QueueDelegate, QueueRead};
use objectiveai_sdk::mcp::queue_notification::{SUFFIX, format_prefix};
use objectiveai_sdk::mcp::tool::{ContentBlock, TextContent};
use test_upstream::{TestTool, TestToolBehavior};

/// Scripted delegate: hands out queued [`QueueRead`]s in order (one
/// per `tools/call`), records every session id it was consulted for.
struct FakeDelegate {
    reads: Mutex<VecDeque<QueueRead>>,
    calls: Mutex<Vec<String>>,
}

impl FakeDelegate {
    fn new(reads: Vec<QueueRead>) -> Arc<Self> {
        Arc::new(Self {
            reads: Mutex::new(reads.into()),
            calls: Mutex::new(Vec::new()),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl QueueDelegate for FakeDelegate {
    fn read_pending_blocks<'a>(
        &'a self,
        _agent_arguments: &'a IndexMap<String, String>,
        mcp_session_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Option<QueueRead>> + Send + 'a>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(mcp_session_id.to_string());
            self.reads.lock().unwrap().pop_front()
        })
    }
}

fn text(text: &str) -> ContentBlock {
    ContentBlock::Text(TextContent {
        text: text.to_string(),
        annotations: None,
        _meta: None,
    })
}

/// Boot the proxy in-process on an ephemeral port with the given
/// delegate; returns its base URL. The serve task dies with the test
/// runtime.
async fn start_proxy(delegate: Option<Arc<dyn QueueDelegate>>) -> String {
    let config = Config {
        address: "127.0.0.1".to_string(),
        port: 0,
        user_agent: format!("objectiveai-mcp-proxy/{}", env!("CARGO_PKG_VERSION")),
        http_referer: "https://objectiveai.dev".to_string(),
        x_title: "ObjectiveAI MCP Proxy".to_string(),
        mcp_connect_timeout: 30000,
        mcp_call_timeout: 30000,
        mcp_backoff_current_interval: 100,
        mcp_backoff_initial_interval: 100,
        mcp_backoff_randomization_factor: 0.5,
        mcp_backoff_multiplier: 1.5,
        mcp_backoff_max_interval: 1000,
        mcp_backoff_max_elapsed_time: 40000,
        mcp_encryption_key: None,
        suppress_output: true,
        logs_dir: None,
    };
    let (listener, app) = objectiveai_mcp_proxy::setup(config, delegate)
        .await
        .expect("proxy setup");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(objectiveai_mcp_proxy::serve(listener, app));
    format!("http://{addr}/")
}

async fn say_upstream() -> Upstream {
    spawn_upstream(UpstreamSpec::new("alpha").with_tools(vec![TestTool {
        name: "say".into(),
        description: None,
        behavior: TestToolBehavior::Static {
            reply: "from-alpha".into(),
        },
    }]))
    .await
}

fn x_mcp_servers(upstream: &Upstream) -> String {
    serde_json::to_string(&[upstream.url.as_str()]).unwrap()
}

#[tokio::test]
async fn delegate_blocks_splice_with_token_wrapper() {
    let upstream = say_upstream().await;
    let delegate = FakeDelegate::new(vec![QueueRead {
        token: "tok-1".into(),
        blocks: vec![vec![text("hello world")]],
    }]);
    let proxy_url = start_proxy(Some(delegate.clone())).await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &proxy_url, &x_mcp_servers(&upstream)).await;

    let response = call_tool(&client, &proxy_url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(
        blocks.len(),
        4,
        "expected wrapper open + queued + wrapper close + tool output, got {blocks:?}"
    );
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[0]["text"], format_prefix("tok-1"));
    assert_eq!(blocks[1]["type"], "text");
    assert_eq!(blocks[1]["text"], "hello world");
    assert_eq!(blocks[2]["type"], "text");
    assert_eq!(blocks[2]["text"], SUFFIX);
    assert_eq!(blocks[3]["type"], "text");
    assert_eq!(blocks[3]["text"], "from-alpha");
    // The block count rides structurally in _meta so consumers don't
    // have to parse the wrapper text.
    assert_eq!(response["result"]["_meta"]["notifications"], 1);
    assert_eq!(delegate.call_count(), 1);
}

#[tokio::test]
async fn multiple_rows_flatten_in_arrival_order() {
    let upstream = say_upstream().await;
    let delegate = FakeDelegate::new(vec![QueueRead {
        token: "tok-2".into(),
        blocks: vec![vec![text("first")], vec![text("second")]],
    }]);
    let proxy_url = start_proxy(Some(delegate.clone())).await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &proxy_url, &x_mcp_servers(&upstream)).await;

    let response = call_tool(&client, &proxy_url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(
        blocks.len(),
        5,
        "one wrapper bracketing BOTH rows oldest-first, got {blocks:?}"
    );
    assert_eq!(blocks[0]["text"], format_prefix("tok-2"));
    assert_eq!(blocks[1]["text"], "first");
    assert_eq!(blocks[2]["text"], "second");
    assert_eq!(blocks[3]["text"], SUFFIX);
    assert_eq!(blocks[4]["text"], "from-alpha");
    assert_eq!(response["result"]["_meta"]["notifications"], 2);
}

#[tokio::test]
async fn drained_delegate_leaves_later_calls_unwrapped() {
    let upstream = say_upstream().await;
    let delegate = FakeDelegate::new(vec![QueueRead {
        token: "tok-3".into(),
        blocks: vec![vec![text("only once")]],
    }]);
    let proxy_url = start_proxy(Some(delegate.clone())).await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &proxy_url, &x_mcp_servers(&upstream)).await;

    let first = call_tool(&client, &proxy_url, &session_id, 2, "alpha_say").await;
    assert_eq!(content_blocks(&first).len(), 4, "first call is wrapped");

    let second = call_tool(&client, &proxy_url, &session_id, 3, "alpha_say").await;
    let blocks = content_blocks(&second);
    assert_eq!(
        blocks.len(),
        1,
        "drained delegate -> bare tool output, got {blocks:?}"
    );
    assert_eq!(blocks[0]["text"], "from-alpha");
    assert!(
        second["result"]["_meta"]["notifications"].is_null(),
        "no notifications meta on an unwrapped response"
    );
    // The delegate is consulted once per successful tools/call.
    assert_eq!(delegate.call_count(), 2);
}

#[tokio::test]
async fn empty_read_means_no_wrapper() {
    let upstream = say_upstream().await;
    let delegate = FakeDelegate::new(vec![QueueRead {
        token: "tok-4".into(),
        blocks: Vec::new(),
    }]);
    let proxy_url = start_proxy(Some(delegate.clone())).await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &proxy_url, &x_mcp_servers(&upstream)).await;

    let response = call_tool(&client, &proxy_url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(blocks.len(), 1, "zero blocks -> no wrapper, got {blocks:?}");
    assert_eq!(blocks[0]["text"], "from-alpha");
    assert!(response["result"]["_meta"]["notifications"].is_null());
}

#[tokio::test]
async fn no_delegate_means_bare_tool_output() {
    let upstream = say_upstream().await;
    let proxy_url = start_proxy(None).await;

    let client = reqwest::Client::new();
    let session_id = init_session(&client, &proxy_url, &x_mcp_servers(&upstream)).await;

    let response = call_tool(&client, &proxy_url, &session_id, 2, "alpha_say").await;
    let blocks = content_blocks(&response);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["text"], "from-alpha");
}

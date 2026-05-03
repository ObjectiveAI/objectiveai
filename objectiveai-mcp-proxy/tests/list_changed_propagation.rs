//! End-to-end:
//! - Spawn an upstream with one tool.
//! - Connect a custom rmcp client that listens for
//!   `notifications/tools/list_changed`.
//! - Hit the upstream's `/__test/set-tools` endpoint to swap tools.
//! - The upstream broadcasts `notifications/tools/list_changed` to its
//!   session; the proxy forwards it onto its downstream SSE GET stream;
//!   our client's `on_tool_list_changed` callback fires.
//! - A subsequent `list_all_tools` returns the new set.

mod common;

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use common::{TestRig, UpstreamSpec};
use test_upstream::{TestTool, TestToolBehavior};
use rmcp::ServiceExt;
use rmcp::handler::client::ClientHandler;
use rmcp::model::ClientInfo;
use rmcp::service::NotificationContext;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use tokio::sync::mpsc;

fn echo_tool(name: &str) -> TestTool {
    TestTool {
        name: name.into(),
        description: None,
        behavior: TestToolBehavior::Echo,
    }
}

#[derive(Clone)]
struct NotifyingClient {
    info: ClientInfo,
    tools_changed_tx: mpsc::UnboundedSender<()>,
}

impl ClientHandler for NotifyingClient {
    async fn on_tool_list_changed(&self, _ctx: NotificationContext<rmcp::RoleClient>) {
        let _ = self.tools_changed_tx.send(());
    }
    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}

#[tokio::test]
async fn upstream_tools_change_reaches_downstream_client() {
    let rig = TestRig::new(vec![
        UpstreamSpec::new("svc").with_tools(vec![echo_tool("alpha")]),
    ])
    .await;

    let (tools_changed_tx, mut tools_changed_rx) = mpsc::unbounded_channel();
    let info: ClientInfo = serde_json::from_value(serde_json::json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": { "name": "test", "version": "0" },
    }))
    .unwrap();
    let handler = NotifyingClient { info, tools_changed_tx };

    let mut headers: HashMap<reqwest::header::HeaderName, reqwest::header::HeaderValue> =
        HashMap::new();
    headers.insert(
        reqwest::header::HeaderName::from_static("x-mcp-servers"),
        reqwest::header::HeaderValue::from_str(&rig.x_mcp_servers()).unwrap(),
    );
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(rig.proxy.url.clone())
            .custom_headers(headers),
    );
    let client = handler.serve(transport).await.expect("client serve");

    // Sanity: initial listing shows just `svc_alpha`.
    let initial: HashSet<String> = client
        .peer()
        .list_all_tools()
        .await
        .expect("initial list_all_tools")
        .into_iter()
        .map(|t| t.name.into())
        .collect();
    assert_eq!(initial, HashSet::from(["svc_alpha".to_string()]));

    // Mutate upstream — this fires `notifications/tools/list_changed`
    // toward the proxy, which fires it toward us.
    rig.set_upstream_tools(0, vec![echo_tool("beta"), echo_tool("gamma")])
        .await;

    // Wait for our handler to get the notification.
    tokio::time::timeout(Duration::from_secs(5), tools_changed_rx.recv())
        .await
        .expect("notifications/tools/list_changed never reached the client")
        .expect("notification channel closed");

    // Re-list — must see the new set.
    let after: HashSet<String> = client
        .peer()
        .list_all_tools()
        .await
        .expect("re-list_all_tools")
        .into_iter()
        .map(|t| t.name.into())
        .collect();
    assert_eq!(
        after,
        HashSet::from(["svc_beta".to_string(), "svc_gamma".to_string()])
    );

    client.cancel().await.ok();
}

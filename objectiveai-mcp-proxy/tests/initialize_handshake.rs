//! Smoke test that proves the rig works end-to-end:
//! - Spawn one upstream and the proxy.
//! - Connect an rmcp client through the proxy.
//! - The connection's `initialize` handshake completes (otherwise
//!   `serve(transport).await` would error out).

mod common;

use std::collections::HashMap;

use common::{TestRig, UpstreamSpec};

#[tokio::test]
async fn initialize_handshake_succeeds() {
    let rig = TestRig::new(vec![UpstreamSpec::new("upstream-a")]).await;

    let mut headers = HashMap::new();
    headers.insert("X-MCP-Servers", rig.x_mcp_servers());

    let client = rig.connect_client(headers).await;

    // Sanity: the rmcp client got a peer info from the proxy's
    // initialize response, which means the handshake actually completed.
    assert!(client.peer_info().is_some(), "peer info missing — initialize never completed");

    // Tear down cleanly.
    client.cancel().await.ok();
}

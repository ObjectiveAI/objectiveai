use std::future::Future;

/// Reads the on-disk config and hands a remote-mode HttpClient to
/// the task closure. The cli always treats the api as externally
/// running; configure its address/port/headers via the
/// `objectiveai api …` subcommands (or the matching env vars
/// listed in `api/client.rs`).
///
/// `cli_config` is threaded into `build_http_client` so the agent_id
/// (env or per-request override from an embedder) reaches outbound
/// `X-OBJECTIVEAI-AGENT-ID`.
pub async fn run<F, Fut>(cli_config: &crate::Config, task: F) -> Result<(), crate::error::Error>
where
    F: FnOnce(objectiveai_sdk::HttpClient) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), crate::error::Error>> + Send + 'static,
{
    let client = objectiveai_sdk::filesystem::Client::new(
        None::<String>,
        None::<String>,
        None::<String>,
    );
    let mut config = client.read_config().await?;
    let http_client = crate::api::client::build_http_client(cli_config, &mut config);
    task(http_client).await
}

/// Variant of [`run`] for streaming endpoints: in addition to the
/// HttpClient, builds a [`crate::api::conduit::ConduitMcpHandler`]
/// from the same config (`config.mcp().get_address()` /
/// `get_port()`, env-var overridable via `OBJECTIVEAI_MCP_ADDRESS` /
/// `OBJECTIVEAI_MCP_PORT`). The handler dials the configured remote
/// MCP on the first inbound `server_request`; if no MCP is
/// configured every request 501s.
pub async fn run_with_conduit<F, Fut>(cli_config: &crate::Config, task: F) -> Result<(), crate::error::Error>
where
    F: FnOnce(
            objectiveai_sdk::HttpClient,
            crate::api::conduit::ConduitMcpHandler,
        ) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = Result<(), crate::error::Error>> + Send + 'static,
{
    let client = objectiveai_sdk::filesystem::Client::new(
        None::<String>,
        None::<String>,
        None::<String>,
    );
    let mut config = client.read_config().await?;
    let http_client = crate::api::client::build_http_client(cli_config, &mut config);
    let conduit = crate::api::conduit::build_handler(&mut config);
    task(http_client, conduit).await
}

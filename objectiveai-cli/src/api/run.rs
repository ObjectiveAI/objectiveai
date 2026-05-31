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
///
/// Streaming subcommands no longer call this — they spawn
/// `objectiveai-cli-stream` via [`super::stream_subprocess::run`]
/// which holds its own HttpClient + MCP conduit.
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

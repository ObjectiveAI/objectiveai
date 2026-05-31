use clap::Args as ClapArgs;

/// `POST /vector/completions` — unary only.
///
/// Streaming vector completions go over WebSocket (with reverse-attach
/// for `client_objectiveai_mcp`), which lives in `objectiveai-cli-stream`
/// now. This cli's `api vector completions post` no longer supports
/// `stream: true` — it'll reject the body. Use cli-stream directly
/// for the streaming path (when added).
#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    pub body: crate::api::body::BodySource,
    #[command(flatten)]
    pub agent_id: crate::api::agent_id_arg::AgentIdArg,
}

pub async fn handle(args: Args, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
    let params: objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams = args.body.resolve()?;
    if params.stream.unwrap_or(false) {
        return Err(crate::error::Error::MissingArgs(
            "streaming vector completions are not supported via `api vector completions post`; use objectiveai-cli-stream",
        ));
    }
    crate::api::call::call_unary::<_, serde_json::Value>(
        cli_config, handle, reqwest::Method::POST, "vector/completions", Some(params),
        args.agent_id.agent_id,
    ).await
}

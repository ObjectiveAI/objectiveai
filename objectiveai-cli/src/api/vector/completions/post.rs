use clap::Args as ClapArgs;

/// `POST /vector/completions` — dual-mode.
///
/// Streaming requests always go over WebSocket (with the CLI's
/// `ConduitMcpHandler` attached as the reverse-attach handler) so
/// that any agent declaring `client_objectiveai_mcp` gets a live
/// proxy URL bridged back to the local MCP. Unary requests stay on
/// plain HTTP.
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
        crate::api::call::call_streaming_ws::<_, objectiveai_sdk::vector::completions::response::streaming::VectorCompletionChunk>(
            cli_config, handle, reqwest::Method::POST, "vector/completions", params,
            args.agent_id.agent_id,
        ).await
    } else {
        crate::api::call::call_unary::<_, serde_json::Value>(
            cli_config, handle, reqwest::Method::POST, "vector/completions", Some(params),
            args.agent_id.agent_id,
        ).await
    }
}

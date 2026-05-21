use clap::Args as ClapArgs;

/// `POST /functions/inventions` — dual-mode.
#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    pub body: crate::api::body::BodySource,
    #[command(flatten)]
    pub agent_id: crate::api::agent_id_arg::AgentIdArg,
}

pub async fn handle(args: Args, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
    let params: objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams = args.body.resolve()?;
    if params.stream.unwrap_or(false) {
        crate::api::call::call_streaming::<_, objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk>(
            cli_config, handle, reqwest::Method::POST, "functions/inventions", Some(params),
            args.agent_id.agent_id,
        ).await
    } else {
        crate::api::call::call_unary::<_, serde_json::Value>(
            cli_config, handle, reqwest::Method::POST, "functions/inventions", Some(params),
            args.agent_id.agent_id,
        ).await
    }
}

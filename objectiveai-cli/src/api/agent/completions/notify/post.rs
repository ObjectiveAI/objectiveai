use clap::Args as ClapArgs;

/// `POST /agent/completions/notify` — queue a user message onto a
/// running agent completion's MCP-proxy notify queue, identified by
/// `response_id`. The agent picks it up on its next tool call.
#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    pub body: crate::api::body::BodySource,
    #[command(flatten)]
    pub agent_id: crate::api::agent_id_arg::AgentIdArg,
}

pub async fn handle(args: Args, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
    let params: objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams = args.body.resolve()?;
    crate::api::call::call_unary_no_response::<_>(
        cli_config, handle, reqwest::Method::POST, "agent/completions/notify", Some(params),
        args.agent_id.agent_id,
    ).await
}

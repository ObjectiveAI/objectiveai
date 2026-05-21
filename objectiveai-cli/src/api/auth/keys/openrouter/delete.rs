use clap::Args as ClapArgs;

/// `DELETE /auth/keys/openrouter`
#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    pub agent_id: crate::api::agent_id_arg::AgentIdArg,
}

pub async fn handle(args: Args, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
    crate::api::call::call_unary::<(), serde_json::Value>(
        cli_config, handle, reqwest::Method::DELETE, "auth/keys/openrouter", None,
        args.agent_id.agent_id,
    ).await
}

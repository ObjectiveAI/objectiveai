use clap::Args as ClapArgs;

/// `POST /swarms/list`
#[derive(ClapArgs)]
pub struct Args {
    #[command(flatten)]
    pub body: crate::api::body::BodySource,
    #[command(flatten)]
    pub agent_id: crate::api::agent_id_arg::AgentIdArg,
}

pub async fn handle(args: Args, cli_config: &crate::Config, handle: &objectiveai_sdk::cli::output::Handle) -> Result<(), crate::error::Error> {
    let req: objectiveai_sdk::swarm::request::ListSwarmsRequest = args.body.resolve()?;
    crate::api::call::call_unary::<_, serde_json::Value>(
        cli_config, handle, reqwest::Method::POST, "swarms/list", Some(req),
        args.agent_id.agent_id,
    ).await
}

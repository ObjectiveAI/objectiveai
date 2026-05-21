#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let cli_config = objectiveai_cli::load_config();
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    // Default destination: this process's stdout. Programmatic embedders
    // constructing their own `cli::run` call can supply a `Handle` whose
    // `destination` is `HandleDestination::Stdin(...)` or
    // `HandleDestination::Collect(_)` instead.
    //
    // Stamp the handle's agent_id from cli_config so every emitted
    // notification + error line carries `X-OBJECTIVEAI-AGENT-ID`'s
    // value (env `OBJECTIVEAI_AGENT_ID`). Per-request callers like
    // objectiveai-mcp-cli build their own Handle and set this from
    // the X-OBJECTIVEAI-AGENT-ID request header instead.
    let mut handle = objectiveai_sdk::cli::output::Handle::stdout();
    handle.agent_id = cli_config.agent_id.clone();
    let code = objectiveai_cli::run(args, &cli_config, handle).await;
    std::process::exit(code);
}

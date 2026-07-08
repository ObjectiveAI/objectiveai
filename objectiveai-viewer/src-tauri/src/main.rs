#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use envconfig::Envconfig;

/// The viewer shell. Configuration comes from the environment
/// (DAEMON_ADDRESS et al — see `run.rs`); the one CLI argument scopes
/// the window.
#[derive(Parser)]
struct Args {
    /// Open ONLY the agent conversation window for this AIH — the
    /// main window never opens. The fast path for debugging one
    /// agent's conversation UI.
    #[arg(long)]
    agent_instance_hierarchy: Option<String>,
}

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let args = Args::parse();
    let mut config = objectiveai_viewer::ConfigBuilder::init_from_env().unwrap().build();
    config.agent_instance_hierarchy = args.agent_instance_hierarchy;
    let code = objectiveai_viewer::run(config).await.unwrap();
    std::process::exit(code);
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    // All configuration comes from the environment (DAEMON_ADDRESS et
    // al — see `run.rs`); there are no CLI arguments.
    let config = objectiveai_viewer::ConfigBuilder::init_from_env()
        .unwrap()
        .build();
    let code = objectiveai_viewer::run(config).await.unwrap();
    std::process::exit(code);
}

use envconfig::Envconfig;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _ = dotenv::dotenv();
    let config = objectiveai_mcp::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();
    let mut executor = BinaryExecutor::new(config.objectiveai_dir.clone());
    if let Some(dir) = &config.objectiveai_dir {
        executor = executor.env("OBJECTIVEAI_DIR", dir.clone());
    }
    if let Some(state) = &config.objectiveai_state {
        executor = executor.env("OBJECTIVEAI_STATE", state.clone());
    }
    objectiveai_mcp::run(config, executor).await
}

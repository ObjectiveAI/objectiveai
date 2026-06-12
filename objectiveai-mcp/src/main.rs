use envconfig::Envconfig;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _ = dotenv::dotenv();
    let config = objectiveai_mcp::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();
    let executor = BinaryExecutor::new(Some(config.objectiveai_dir.clone()))
        .env(
            "OBJECTIVEAI_DIR",
            config.objectiveai_dir.to_string_lossy().into_owned(),
        )
        .env("OBJECTIVEAI_STATE", config.objectiveai_state.clone());
    objectiveai_mcp::run(config, executor).await
}

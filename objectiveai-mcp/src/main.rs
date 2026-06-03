use envconfig::Envconfig;
use objectiveai_sdk::cli::command::binary::BinaryExecutor;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let _ = dotenv::dotenv();
    let config = objectiveai_mcp::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();
    let executor = BinaryExecutor::new(config.config_base_dir.clone());
    objectiveai_mcp::run(config, executor).await
}

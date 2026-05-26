use envconfig::Envconfig;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let _ = dotenv::dotenv();
    let config = objectiveai_mcp::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();

    objectiveai_mcp::run(config).await
}

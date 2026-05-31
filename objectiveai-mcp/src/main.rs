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

    // Marker for every descendant process: "you are running under
    // objectiveai-mcp". Set before `dotenv::dotenv()` so a .env file
    // can't accidentally override it, and before any subprocess is
    // spawned (Command inherits env by default, so children see this
    // even without explicit propagation). The cli further re-emits it
    // explicitly on plugin / tool spawn sites — see
    // `objectiveai-cli/src/{plugins,tools}/mod.rs`.
    //
    // SAFETY: set_var is sound at this point because main hasn't
    // spawned any worker threads yet and dotenv (which races for
    // env access) is the only follow-up reader.
    unsafe {
        std::env::set_var(objectiveai_sdk::mcp::OBJECTIVEAI_MCP_ENV, "true");
    }

    let _ = dotenv::dotenv();
    let config = objectiveai_mcp::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();

    objectiveai_mcp::run(config).await
}

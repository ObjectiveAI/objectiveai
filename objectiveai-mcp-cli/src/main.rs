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
    // Best-effort auto-update. Compiled out entirely unless the
    // `updater` feature is on; may never return (re-execs the
    // replacement). No Handle — the updater falls back to `println!`.
    // (The mcp-cli binary is invoked via stdio MCP transport, so the
    // updater's stdout-JSONL becomes part of the MCP server's
    // pre-startup output — fine.)
    #[cfg(feature = "updater")]
    {
        let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
        objectiveai_sdk::updater::maybe_auto_update(
            objectiveai_sdk::updater::UpdaterConfig {
                asset_prefix: "objectiveai-mcp",
                variant_suffix: "",
                current_version: env!("CARGO_PKG_VERSION"),
                github_authorization: None,
                handle: None,
            },
            args,
        )
        .await;
    }
    let config = objectiveai_mcp_cli::ConfigBuilder::init_from_env()
        .unwrap_or_default()
        .build();

    objectiveai_mcp_cli::run(config).await
}

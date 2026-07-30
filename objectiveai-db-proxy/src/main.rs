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

    // No configuration to read — not from arguments, not from the
    // environment, not from a `.env`. See `run.rs` for why a binary that
    // gets `podman exec`'d into somebody else's image deliberately has
    // no knobs.
    objectiveai_db_proxy::run().await
}

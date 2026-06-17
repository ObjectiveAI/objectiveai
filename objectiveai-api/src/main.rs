use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    // Before anything else: if this process was launched as a
    // subprocess-reaper guardian (macOS only), run its kqueue watch loop
    // and `process::exit`. A no-op on every other platform and on every
    // normal invocation. Must come first — the guardian re-execs THIS
    // binary, so it must not fall through into the api server.
    objectiveai_sdk::subprocess_reaper::run_guardian_if_invoked();

    // Two-tier dotenv: the regular CWD `.env` overrides
    // `<OBJECTIVEAI_DIR>/.env` (the repo's committed `.objectiveai/.env`
    // carries the test-grade settings). dotenv never overrides an
    // already-set var, so loading the CWD file FIRST makes it win over
    // the dir-scoped file, and the real environment still wins over both.
    let dir = std::env::var_os("OBJECTIVEAI_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".objectiveai")
        });
    let _ = dotenv::dotenv();
    let _ = dotenv::from_path(dir.join(".env"));
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();
    objectiveai_api::run(config).await.unwrap();
}

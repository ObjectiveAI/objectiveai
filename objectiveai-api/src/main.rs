use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    // Layout-scoped dotenv first: `<OBJECTIVEAI_DIR>/.env` (the repo's
    // committed `.objectiveai/.env` carries the test-grade settings).
    // dotenv never overrides variables that are already set, so real
    // env always wins, and the dir-scoped file beats the CWD one.
    let dir = std::env::var_os("OBJECTIVEAI_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".objectiveai")
        });
    let _ = dotenv::from_path(dir.join(".env"));
    let _ = dotenv::dotenv();
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();
    objectiveai_api::run(config).await.unwrap();
}

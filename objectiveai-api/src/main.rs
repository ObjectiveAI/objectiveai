use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    // Best-effort auto-update. Compiled out entirely unless the
    // `updater` feature is on; may never return (re-execs the
    // replacement binary). With no Handle, the updater falls back to
    // `println!`/`eprintln!`, equivalent to `Handle::Stdout`.
    #[cfg(feature = "updater")]
    {
        // `ArgsOs` isn't `Clone`, and the updater needs an
        // `Iterator + Clone` so it can forward the same argv to the
        // re-exec'd replacement binary.
        let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
        objectiveai_sdk::updater::maybe_auto_update(
            objectiveai_sdk::updater::UpdaterConfig {
                asset_prefix: "objectiveai-api",
                variant_suffix: "",
                current_version: env!("CARGO_PKG_VERSION"),
                github_authorization: None,
                handle: None,
            },
            args,
        )
        .await;
    }
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();
    objectiveai_api::run(config).await.unwrap();
}

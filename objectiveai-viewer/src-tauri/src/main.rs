#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    // Best-effort auto-update. Compiled out entirely unless the
    // `updater` feature is on (off for the embedded-in-cli viewer
    // build, on for the standalone release). May never return
    // (re-execs the replacement). No Handle — updater falls back to
    // `println!`/`eprintln!`.
    #[cfg(feature = "updater")]
    {
        let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
        objectiveai_sdk::updater::maybe_auto_update(
            objectiveai_sdk::updater::UpdaterConfig {
                asset_prefix: "objectiveai-viewer",
                variant_suffix: "",
                current_version: env!("CARGO_PKG_VERSION"),
                github_authorization: None,
                handle: None,
            },
            args,
        )
        .await;
    }
    let config = objectiveai_viewer::ConfigBuilder::init_from_env().unwrap().build();
    let code = objectiveai_viewer::run(config).await.unwrap();
    std::process::exit(code);
}

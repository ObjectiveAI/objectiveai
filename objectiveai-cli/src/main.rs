#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    // Build the top-level config ONCE after `.env` is loaded, so the
    // auto-updater and `run` share the same env-sourced view (notably
    // `GITHUB_AUTHORIZATION`, used by the updater).
    let cli_config = objectiveai_cli::load_config();
    // Collect argv once so the updater can forward it to the re-exec'd
    // new binary while run() still gets the same sequence.
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    // Default destination: this process's stdout. Programmatic embedders
    // constructing their own `cli::run` call can supply
    // `Handle::Stdin(Arc::new(Mutex::new(child.stdin.take().unwrap())))`
    // or `Handle::Collect(_)` instead.
    let handle: objectiveai_sdk::cli::output::Handle =
        objectiveai_sdk::cli::output::Handle::Stdout;
    // Best-effort auto-update. Compiled out entirely unless the
    // `updater` feature is on; may never return because the replacement
    // has been spawned with the same argv. Errors are reported as a
    // warn-level `Output::Error` line through the shared handle.
    #[cfg(feature = "updater")]
    objectiveai_sdk::updater::maybe_auto_update(
        objectiveai_sdk::updater::UpdaterConfig {
            asset_prefix: "objectiveai",
            variant_suffix: if cfg!(feature = "viewer") { "" } else { "-no-viewer" },
            current_version: env!("CARGO_PKG_VERSION"),
            github_authorization: cli_config.github_authorization.clone(),
            // Share the cli's JSONL stream so updater notifications
            // appear inline with normal `cli::run` output.
            handle: Some(handle.clone()),
        },
        args.clone(),
    )
    .await;
    let code = objectiveai_cli::run(args, &cli_config, handle).await;
    std::process::exit(code);
}

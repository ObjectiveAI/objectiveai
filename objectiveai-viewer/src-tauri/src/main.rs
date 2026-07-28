#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use envconfig::Envconfig;

fn main() {
    // FIRST, before the tokio runtime, the dotenv load, or anything
    // else: Chromium re-executes this very binary for each of its
    // helper processes (renderer, GPU, utility, ...), marked by a
    // `--type=` switch. A helper must hand straight off to CEF and
    // exit — it is not a viewer, and everything below (not least the
    // daemon readiness line the daemon is blocked reading) would be
    // wrong to run several times over. See `crate::cef`.
    #[cfg(not(target_os = "macos"))]
    if objectiveai_viewer::cef::is_helper_process() {
        objectiveai_viewer::cef::run_helper_and_exit();
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let code = runtime.block_on(async {
        let _ = dotenv::dotenv();
        // All configuration comes from the environment (DAEMON_ADDRESS
        // et al — see `run.rs`); there are no CLI arguments of our own.
        let config = objectiveai_viewer::ConfigBuilder::init_from_env()
            .unwrap()
            .build();
        objectiveai_viewer::run(config).await.unwrap()
    });
    std::process::exit(code);
}

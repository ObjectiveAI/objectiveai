use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();

    // Claim key `<ADDRESS>_<PORT>` in `<OBJECTIVEAI_DIR>/bin/locks/
    // api/` for the lifetime of this process — "an api server for
    // this address:port is alive" becomes kernel-observable
    // (lockfile semantics: released on ANY exit, crash included).
    // The SDK owns dir creation, key escaping, and the `.lock`
    // suffix. Best-effort by design: acquisition failure is ignored,
    // the result is discarded, and nothing in here consults the
    // claim again.
    if let Some(claim) = objectiveai_sdk::lockfile::try_acquire(
        &config.objectiveai_dir.join("bin").join("locks").join("api"),
        &format!("{}_{}", config.address, config.port),
    )
    .await
    {
        // Never dropped in-process: the kernel releases the claim at
        // process death, which is exactly the lifetime we want.
        std::mem::forget(claim);
    }

    objectiveai_api::run(config).await.unwrap();
}

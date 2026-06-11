use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();

    // Claim key `<ADDRESS>_<PORT>` in `<OBJECTIVEAI_DIR>/bin/locks/
    // api/` — "an api server for this address:port is alive" becomes
    // kernel-observable. Discarding the result IS the hold: a
    // LockClaim leaks its handle on drop by design, so the claim
    // ends only at process death (crash included). Acquisition
    // failure is ignored, best-effort by design.
    let _ = objectiveai_sdk::lockfile::try_acquire(
        &config.objectiveai_dir.join("bin").join("locks").join("api"),
        &format!("{}_{}", config.address, config.port),
    )
    .await;

    objectiveai_api::run(config).await.unwrap();
}

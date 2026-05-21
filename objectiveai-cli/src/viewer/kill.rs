/// `objectiveai viewer kill` — terminate every running
/// `objectiveai-viewer` process. Idempotent: a count of zero is not an
/// error.
pub async fn handle(
    _cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
    let count = crate::spawn::kill_by_name("objectiveai-viewer");
    crate::config::emit_value(&count, handle).await;
    Ok(())
}

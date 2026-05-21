/// `objectiveai api spawn` — start the `objectiveai-api` server in the
/// background using `api.address` + `api.port` from config.
pub async fn handle(
    cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
    let (client, mut config) = crate::config::read(cli_config).await?;
    let address = config
        .api()
        .get_address()
        .ok_or(crate::error::Error::MissingArgs(
            "api.address unset; run `objectiveai api address config set <addr>`",
        ))?
        .to_string();
    let port = config
        .api()
        .get_port()
        .ok_or(crate::error::Error::MissingArgs(
            "api.port unset; run `objectiveai api port config set <port>`",
        ))?;

    crate::spawn::ensure_not_running("objectiveai-api")?;

    let bin = if cfg!(windows) {
        "objectiveai-api.exe"
    } else {
        "objectiveai-api"
    };
    let exe = client.base_dir().join("bin").join(bin);

    let line = crate::spawn::spawn_and_wait_for_listening(&exe, &address, port).await?;
    crate::config::emit_value(&line, handle).await;
    Ok(())
}

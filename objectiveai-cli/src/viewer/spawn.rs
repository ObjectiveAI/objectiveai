/// `objectiveai viewer spawn` — start the `objectiveai-viewer` Tauri
/// shell in the background using `viewer.address` + `viewer.port` from
/// config.
pub async fn handle(
    cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
    let (client, mut config) = crate::config::read(cli_config).await?;
    let address = config
        .viewer()
        .get_address()
        .ok_or(crate::error::Error::MissingArgs(
            "viewer.address unset; run `objectiveai viewer address config set <addr>`",
        ))?
        .to_string();
    let port = config
        .viewer()
        .get_port()
        .ok_or(crate::error::Error::MissingArgs(
            "viewer.port unset; run `objectiveai viewer port config set <port>`",
        ))?;

    crate::spawn::ensure_not_running("objectiveai-viewer")?;

    let bin = if cfg!(windows) {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };
    let exe = client.base_dir().join("bin").join(bin);

    let line = crate::spawn::spawn_and_wait_for_listening(&exe, &address, port).await?;
    crate::config::emit_value(&line, handle).await;
    Ok(())
}

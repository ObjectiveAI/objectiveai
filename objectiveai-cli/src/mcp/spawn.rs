/// `objectiveai mcp spawn` — start the `objectiveai-mcp` server in the
/// background using `mcp.address` + `mcp.port` from config.
pub async fn handle(
    cli_config: &crate::Config,
    handle: &objectiveai_sdk::cli::output::Handle,
) -> Result<(), crate::error::Error> {
    let (client, mut config) = crate::config::read(cli_config).await?;
    let address = config
        .mcp()
        .get_address()
        .ok_or(crate::error::Error::MissingArgs(
            "mcp.address unset; run `objectiveai mcp address config set <addr>`",
        ))?
        .to_string();
    let port = config
        .mcp()
        .get_port()
        .ok_or(crate::error::Error::MissingArgs(
            "mcp.port unset; run `objectiveai mcp port config set <port>`",
        ))?;

    crate::spawn::ensure_not_running("objectiveai-mcp")?;

    let bin = if cfg!(windows) {
        "objectiveai-mcp.exe"
    } else {
        "objectiveai-mcp"
    };
    let exe = client.base_dir().join("bin").join(bin);

    let line = crate::spawn::spawn_and_wait_for_listening(&exe, &address, port).await?;
    crate::config::emit_value(&line, handle).await;
    Ok(())
}

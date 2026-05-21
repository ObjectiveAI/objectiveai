use objectiveai_sdk::filesystem::config::Config;
use objectiveai_sdk::filesystem::Client;
use objectiveai_sdk::cli::output::{Handle, JqResults, Output, Value};

pub fn filter(f: Option<String>) -> String {
    f.unwrap_or_else(|| ".".to_string())
}

pub async fn read(cli_config: &super::Config) -> Result<(Client, Config), crate::error::Error> {
    let client = Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let config = client.read_config().await?;
    Ok((client, config))
}

pub async fn write(client: &Client, config: &Config, cli_config: &super::Config) -> Result<(), crate::error::Error> {
    if cli_config.config_set_forbidden {
        return Err(crate::error::Error::ConfigSetForbidden);
    }
    client.write_config(config).await?;
    Ok(())
}

/// Emit a user-supplied jq filter's results as a single `Notification::Jq`.
pub async fn emit_jq(
    results: Result<Vec<serde_json::Value>, objectiveai_sdk::filesystem::Error>,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let results = results?;
    let jq = match results.len() {
        0 => serde_json::Value::Null,
        1 => results.into_iter().next().unwrap(),
        _ => serde_json::Value::Array(results),
    };
    Output::<JqResults>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: JqResults { jq } })
        .emit(handle)
        .await;
    Ok(())
}

/// Emit a typed config value as `{"type":"notification","value":<v>}`.
pub async fn emit_value<V: serde::Serialize>(v: V, handle: &Handle) {
    Output::<Value<V>>::Notification(objectiveai_sdk::cli::output::Notification { agent_id: None, value: Value { value: v } })
        .emit(handle)
        .await;
}

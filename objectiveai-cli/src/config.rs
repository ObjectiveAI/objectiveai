use objectiveai::filesystem::config::Config;
use objectiveai::filesystem::Client;

pub fn filter(f: Option<String>) -> String {
    f.unwrap_or_else(|| ".".to_string())
}

pub async fn read(cli_config: &super::Config) -> Result<(Client, Config), crate::error::Error> {
    let client = Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let config = objectiveai::filesystem::config::client::read(&client).await?;
    Ok((client, config))
}

pub async fn write(client: &Client, config: &Config, cli_config: &super::Config) -> Result<(), crate::error::Error> {
    if cli_config.config_set_forbidden {
        return Err(crate::error::Error::ConfigSetForbidden);
    }
    objectiveai::filesystem::config::client::write(client, config).await?;
    Ok(())
}

/// Wire shape for a user-supplied jq filter's results. Documented escape
/// hatch — jq produces arbitrary JSON that cannot be typed in advance.
#[derive(serde::Serialize)]
pub struct JqResults {
    pub jq: serde_json::Value,
}

pub fn emit_jq(
    results: Result<Vec<serde_json::Value>, objectiveai::filesystem::Error>,
) -> Result<(), crate::error::Error> {
    let results = results?;
    let jq = match results.len() {
        0 => serde_json::Value::Null,
        1 => results.into_iter().next().unwrap(),
        _ => serde_json::Value::Array(results),
    };
    objectiveai_cli_lib::output::Output::<JqResults>::Notification(JqResults { jq }).emit();
    Ok(())
}

/// Emits a typed config value as `{"type":"notification","value":<v>}`.
pub fn emit_value<V: serde::Serialize>(v: V) {
    #[derive(serde::Serialize)]
    struct Value<V: serde::Serialize> {
        value: V,
    }
    objectiveai_cli_lib::output::Output::<Value<V>>::Notification(Value { value: v }).emit();
}

//! Layer selectors required by every `config` command. Reads take
//! exactly one of `--global` / `--state` / `--final`; mutations take
//! exactly one of `--global` / `--state`. Semantics land in a later
//! step — for now the flags are parsed, validated (exactly one), and
//! carried on every Request.

/// Which config layer a read targets.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.command.config.GetScope")]
#[serde(rename_all = "snake_case")]
pub enum GetScope {
    Global,
    State,
    Final,
}

/// Which config layer a mutation targets.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.command.config.SetScope")]
#[serde(rename_all = "snake_case")]
pub enum SetScope {
    Global,
    State,
}

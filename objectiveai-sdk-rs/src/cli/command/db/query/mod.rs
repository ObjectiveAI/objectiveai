//! `db query` — execute arbitrary single-statement read-only SQL
//! against the CLI's local postgres pool. Returns the row set as
//! typed JSON cells (Postgres → JSON via a per-cell decoder),
//! with column metadata and the wire-protocol command tag.
//!
//! Constraints:
//! - **Read-only**: wrapped in `SET LOCAL TRANSACTION READ ONLY`
//!   server-side. Write attempts come back as
//!   `Error::QueryReadOnlyViolation`.
//! - **Timeout**: the [`RequestBase`] envelope's optional
//!   `--timeout` (humantime: `30s`, `5m`, `1h30m`). When set, the
//!   CLI threads it to postgres as `SET LOCAL statement_timeout` /
//!   `lock_timeout` — enforcement is the server's alone. When
//!   omitted, the query runs uncapped.
//!
//! [`RequestBase`]: crate::cli::command::RequestBase
//!
//! Multi-statement queries, `COPY … TO STDOUT|STDIN`, and
//! transaction-control verbs (`BEGIN` / `COMMIT` / `ROLLBACK`)
//! are rejected up front by a cheap leading-token scan on the
//! CLI side before the query reaches the database.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.query.Request")]
pub struct Request {
    pub path_type: Path,
    /// SQL statement to execute. Single statement only (multi-
    /// statement input is rejected by the CLI handler).
    pub query: String,
    // Carries the optional `timeout_seconds`; see the module doc.
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.query.Path")]
pub enum Path {
    #[serde(rename = "db/query")]
    DbQuery,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "db".to_string(),
            "query".to_string(),
            "--query".to_string(),
            self.query.clone(),
        ];
        self.base.push_flags(&mut argv);
        argv
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One result column. `r#type` is the Postgres `pg_type.typname`
/// (e.g. `"int8"`, `"text"`, `"jsonb"`, `"timestamptz"`). Callers
/// needing precision/scale/array-element-type can inspect the
/// name; richer typeinfo is deferred.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.query.Column")]
pub struct Column {
    pub name: String,
    pub r#type: String,
}

/// Unary response from `db query`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.query.Response")]
pub struct Response {
    /// Wire-protocol-style command tag — e.g. `"SELECT 5"`,
    /// `"SHOW 1"`, `"EXPLAIN 12"`, `"SET 0"`. Synthesized from
    /// `{leading_keyword} {row_count}` since sqlx 0.8 doesn't
    /// expose the wire-protocol `CommandComplete` tag; close
    /// enough for telemetry / display.
    pub command_tag: String,
    /// Result columns in select-list order. Empty for no-row
    /// statements (SET / LISTEN / DO).
    pub columns: Vec<Column>,
    /// One inner Vec per row, length matches `columns.len()`.
    /// Each cell is a `serde_json::Value` produced by the CLI's
    /// per-cell `pg_value_to_json` decoder. Common type
    /// encodings: text/uuid/timestamps as JSON String, numeric
    /// as String (preserving precision), bytea as base64 String,
    /// json/jsonb passthrough as Value, arrays recurse. Empty
    /// when the statement returns no rows (no-row command OR a
    /// SELECT with no matches).
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Always `false` in the current design. Reserved on the wire
    /// so a future "soft truncation" mode can be added without a
    /// shape break.
    pub truncated: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// SQL statement to execute. Single statement only.
    #[arg(long)]
    pub query: String,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            path_type: Path::DbQuery,
            query: args.query,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;
pub mod response_schema;

//! `db query` — execute arbitrary single-statement read-only SQL
//! against the CLI's local postgres pool. Returns the row set as
//! typed JSON cells (Postgres → JSON via a per-cell decoder),
//! with column metadata and the wire-protocol command tag.
//!
//! Constraints:
//! - **Read-only**: wrapped in `SET LOCAL TRANSACTION READ ONLY`
//!   server-side. Write attempts come back as
//!   `Error::QueryReadOnlyViolation`.
//! - **Timeout**: required, humantime-parsed (`30s`, `5m`,
//!   `1h30m`). `0` is rejected at `TryFrom<Args>` time. The CLI
//!   threads it into both `SET LOCAL statement_timeout` (server)
//!   and `tokio::time::timeout` (client) so cancellation cleanly
//!   drops the connection without leaving the pool poisoned.
//! - **Token budget**: optional `--max-tokens N`. When set, the
//!   CLI tokenizes the response per-part (`command_tag`,
//!   `columns`, each row) via `tiktoken-rs` o200k_base and sums.
//!   Over-budget responses error with `Error::TokenBudgetExceeded
//!   { limit, actual }` carrying the exact count and a
//!   refinement suggestion. `0` is rejected at `TryFrom<Args>`
//!   time (it means "no limit," not "no tokens").
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
    /// Wall-clock execution cap, in whole seconds. Parsed from
    /// `--timeout` (humantime), `> 0` enforced at
    /// `TryFrom<Args>` time so this never carries 0 on the wire.
    pub timeout_seconds: u64,
    /// Optional response token budget. `None` ⇒ no limit;
    /// `Some(n)` requires `n >= 1`. When set, the CLI errors
    /// with `TokenBudgetExceeded` if the per-part token sum
    /// exceeds the limit. Skip-serialized when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_tokens: Option<u64>,
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
            "--timeout".to_string(),
            humantime::format_duration(
                std::time::Duration::from_secs(self.timeout_seconds),
            )
            .to_string(),
        ];
        if let Some(n) = self.max_tokens {
            argv.push("--max-tokens".to_string());
            argv.push(n.to_string());
        }
        self.base.push_flags(&mut argv);
        argv
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
    /// Always `false` in the current design — over-budget
    /// responses return `TokenBudgetExceeded` rather than a
    /// partial result. Reserved on the wire so a future "soft
    /// truncation" mode can be added without a shape break.
    pub truncated: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// SQL statement to execute. Single statement only.
    #[arg(long)]
    pub query: String,
    /// Wall-clock execution cap. Humantime — `100ms`, `30s`,
    /// `5m`, `1h30m`. Must be `> 0`.
    #[arg(long)]
    pub timeout: String,
    /// Optional response token budget. Must be `>= 1` if set
    /// (`0` is rejected; omit the flag entirely to disable
    /// limiting).
    #[arg(long)]
    pub max_tokens: Option<u64>,
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
        let parsed_timeout = humantime::parse_duration(&args.timeout)
            .map_err(|source| crate::cli::command::FromArgsError {
                field: "timeout",
                source: source.to_string().into(),
            })?;
        let timeout_seconds = parsed_timeout.as_secs();
        if timeout_seconds == 0 {
            // Treats sub-second timeouts as 0 too — humantime
            // accepts `100ms` and we'd `.as_secs()` it down to 0.
            // Sub-second is a real use case (cancellation
            // testing); accept it here by rounding up to 1s OR
            // by switching the storage unit. For now, the
            // simplest spec-respecting reject path: require ≥
            // 1s.
            return Err(crate::cli::command::FromArgsError {
                field: "timeout",
                source: "must be >= 1s".to_string().into(),
            });
        }
        if let Some(0) = args.max_tokens {
            return Err(crate::cli::command::FromArgsError {
                field: "max_tokens",
                source: "must be >= 1 (omit the flag for unlimited)"
                    .to_string()
                    .into(),
            });
        }
        Ok(Self {
            path_type: Path::DbQuery,
            query: args.query,
            timeout_seconds,
            max_tokens: args.max_tokens,
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

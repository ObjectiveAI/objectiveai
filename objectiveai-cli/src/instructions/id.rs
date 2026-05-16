//! Shared helpers for the `instructions get` subcommand family and the
//! corresponding `--instructions-id` argument required by every
//! streaming `create` command.
//!
//! Flow:
//! 1. The user runs e.g. `objectiveai agents completions instructions get`
//!    → the CLI mints a fresh Instructions ID, records it against the
//!    matching scope in the config sqlite, and prints the embedded
//!    `INSTRUCTIONS.md` content followed by `\n\n Instructions ID: <id>`.
//! 2. The user passes that ID back to the matching `create` command
//!    via `--instructions-id <ID>`. The command verifies the ID is
//!    present in the same per-scope table before proceeding.
//!
//! Each streaming `create` family gets its own [`InstructionsScope`]
//! (→ its own sqlite table) so an ID issued for one command can't be
//! reused with another. The table shape is trivial: a single
//! `instructions_id TEXT PRIMARY KEY` column, created on demand.

use clap::Args;

/// Per-command scope. Each variant maps to its own sqlite table,
/// isolating IDs between the four streaming `create` families.
#[derive(Clone, Copy, Debug)]
pub enum InstructionsScope {
    AgentCompletions,
    FunctionExecutions,
    FunctionInventionsRecursive,
    LaboratoryExecutions,
}

impl InstructionsScope {
    /// Every scope, in a stable order. Used by `instructions clear` to
    /// iterate the full set of per-scope tables.
    pub const ALL: &'static [Self] = &[
        Self::AgentCompletions,
        Self::FunctionExecutions,
        Self::FunctionInventionsRecursive,
        Self::LaboratoryExecutions,
    ];

    /// Name of the backing sqlite table. Hardcoded per-scope so the
    /// string is always `&'static str` (safe to interpolate into SQL).
    pub fn table_name(self) -> &'static str {
        match self {
            Self::AgentCompletions => "agent_completions_instructions",
            Self::FunctionExecutions => "function_executions_instructions",
            Self::FunctionInventionsRecursive => "function_inventions_recursive_instructions",
            Self::LaboratoryExecutions => "laboratory_executions_instructions",
        }
    }
}

/// Embeddable `--instructions-id <ID>` argument. Flattened into each
/// streaming `create` command's args struct.
#[derive(Args, Clone, Debug)]
pub struct InstructionsIdArg {
    /// ID from the matching `instructions get` subcommand. Required.
    #[arg(long)]
    pub instructions_id: String,
}

impl InstructionsIdArg {
    /// Verify the provided ID was previously issued for this scope.
    ///
    /// Returns [`crate::error::Error::UnknownInstructionsId`] if the
    /// per-scope table is missing or the row is not present. Other
    /// filesystem/sqlite errors propagate as-is.
    pub fn verify(
        &self,
        cli_config: &crate::Config,
        scope: InstructionsScope,
    ) -> Result<(), crate::error::Error> {
        verify(cli_config, scope, &self.instructions_id)
    }
}

/// Mint a new Instructions ID, record it against `scope` in the
/// config sqlite, and return the full subcommand output
/// (`<content>\n\n Instructions ID: <id>`).
///
/// Called by each `instructions get` subcommand handler.
pub fn issue(
    cli_config: &crate::Config,
    scope: InstructionsScope,
    content: &str,
) -> Result<String, crate::error::Error> {
    let client = fs_client(cli_config);
    let id = generate_id();
    let table = scope.table_name();
    // Table creation is idempotent — the `instructions get` subcommand
    // may be the first thing the user ever runs, in which case the
    // sqlite file and this table both need to exist before the INSERT.
    objectiveai_sdk::filesystem::config::db::execute(
        &client,
        &format!(
            "CREATE TABLE IF NOT EXISTS {table} (instructions_id TEXT PRIMARY KEY NOT NULL)"
        ),
        [],
    )?;
    objectiveai_sdk::filesystem::config::db::execute(
        &client,
        &format!("INSERT OR IGNORE INTO {table} (instructions_id) VALUES (?1)"),
        objectiveai_sdk::filesystem::config::db::params![id],
    )?;
    Ok(format!("{content}\n\n Instructions ID: {id}"))
}

/// Check the supplied ID exists for this scope.
///
/// - Returns [`crate::error::Error::UnknownInstructionsId`] when the
///   table is missing (user has never run the matching `instructions
///   get` subcommand on this machine) or when no row matches the given
///   id.
/// - Other db errors (e.g. corruption) propagate unchanged.
pub fn verify(
    cli_config: &crate::Config,
    scope: InstructionsScope,
    id: &str,
) -> Result<(), crate::error::Error> {
    let client = fs_client(cli_config);
    let table = scope.table_name();
    let sql = format!("SELECT 1 FROM {table} WHERE instructions_id = ?1");
    let result = objectiveai_sdk::filesystem::config::db::query_one(
        &client,
        &sql,
        objectiveai_sdk::filesystem::config::db::params![id],
        |row| row.get::<_, i64>(0),
    );
    match result {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(crate::error::Error::UnknownInstructionsId),
        // SQLite surfaces "no such table" as a runtime error; from the
        // user's POV it's indistinguishable from "id missing" so we
        // collapse both into the same variant.
        Err(e) if e.to_string().contains("no such table") => {
            Err(crate::error::Error::UnknownInstructionsId)
        }
        Err(e) => Err(e.into()),
    }
}

/// Drop every per-scope table. Returns the number of tables that were
/// dropped (always equal to `InstructionsScope::ALL.len()` since
/// `DROP TABLE IF EXISTS` is idempotent).
pub fn clear_all(cli_config: &crate::Config) -> Result<usize, crate::error::Error> {
    let client = fs_client(cli_config);
    for scope in InstructionsScope::ALL {
        let table = scope.table_name();
        objectiveai_sdk::filesystem::config::db::execute(
            &client,
            &format!("DROP TABLE IF EXISTS {table}"),
            [],
        )?;
    }
    Ok(InstructionsScope::ALL.len())
}

fn fs_client(cli_config: &crate::Config) -> objectiveai_sdk::filesystem::Client {
    objectiveai_sdk::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    )
}

/// Fresh 128-bit random value → base62-padded to 22 chars. Matches
/// the content-addressed-ID shape used across objectiveai-rs.
fn generate_id() -> String {
    let bits: u128 = uuid::Uuid::new_v4().as_u128();
    format!("{:0>22}", base62::encode(bits))
}

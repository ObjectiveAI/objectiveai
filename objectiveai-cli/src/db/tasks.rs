//! `schedules` table + future task-runner tables. Bodies stubbed; SQL
//! lands in stage 9.

use objectiveai_sdk::cli::command::AgentArguments;

use super::{Error, Pool};

/// One row from `schedules` as surfaced by `agents tasks list`.
#[derive(Debug, Clone)]
pub struct ListedSchedule {
    pub id: i64,
    pub name: String,
    pub agent_instance_hierarchy: String,
    pub command: Vec<String>,
    pub description: String,
    pub created_at: i64,
    pub last_ran_at: Option<i64>,
    pub interval_seconds: Option<u64>,
}

/// Subset of a schedule row that `agents tasks run` needs to fire one
/// task.
#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: i64,
    pub name: String,
    pub command: Vec<String>,
    pub agent_arguments: AgentArguments,
}

/// Insert one schedule row and return its auto-incremented id.
pub async fn insert_schedule(
    _pool: &Pool,
    _name: &str,
    _command: &[String],
    _description: &str,
    _agent_instance_hierarchy: &str,
    _interval_seconds: Option<u64>,
    _agent_arguments: &AgentArguments,
) -> Result<i64, Error> {
    unimplemented!("db::tasks::insert_schedule — stage 9")
}

/// List `schedules` matching the supplied filters.
pub async fn list_schedules(
    _pool: &Pool,
    _parent: &str,
    _max_depth: Option<u64>,
    _oneshot_only: bool,
    _interval_only: bool,
    _pending_only: bool,
    _exhausted_only: bool,
    _offset: u64,
    _count: Option<u64>,
) -> Result<Vec<ListedSchedule>, Error> {
    unimplemented!("db::tasks::list_schedules — stage 9")
}

/// Atomically: capture every pending row in scope, bump every captured
/// row's `last_ran_at = now`, and delete any captured oneshots.
pub async fn collect_and_mark_pending(
    _pool: &Pool,
    _parent: &str,
    _max_depth: Option<u64>,
) -> Result<Vec<RunRow>, Error> {
    unimplemented!("db::tasks::collect_and_mark_pending — stage 9")
}

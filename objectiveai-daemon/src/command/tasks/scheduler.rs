//! The resident TASK SCHEDULER — the live half of the tasks feature
//! (the durable half is [`crate::db::tasks`]).
//!
//! ONE event-driven driver task, no polling: it sleeps until the
//! earliest `next_run_at` (or parks when nothing is scheduled) and is
//! woken by a [`tokio::sync::Notify`] whenever `tasks create` /
//! `tasks delete` / a run completion changes the schedule (`notify`
//! stores a permit, so a wakeup racing the park is never lost).
//!
//! Firing is an ATOMIC DB claim (`scheduled` → `running`; see the
//! schema comment) — the claim is what serializes concurrent
//! schedulers, so a task can never double-fire or overlap itself.
//! Each claimed task runs on its own spawned task (the driver never
//! blocks on runs): the stored identity is rebuilt
//! ([`ScopeIdentity::from_agent_arguments`] + [`ScopedContext::with_plugin`]
//! when the stored trio is whole + [`ScopedContext::with_task`] —
//! the ONE place the `task` identity flag is set), the stored command
//! rides the `--request` front door exactly like
//! [`crate::command::detached`], and the run's outcome is its LAST
//! stream item — `Err` = errored. Completion is a second atomic
//! UPDATE (counters + re-arm/complete), then a notify so the driver
//! re-scans.
//!
//! Boot: rows stranded in `running` (a crashed daemon's in-flight
//! runs) are reconciled back to `scheduled`, due immediately —
//! at-least-once execution. DB unavailability never kills the driver;
//! it backs off and retries.

use std::sync::Arc;

use futures::StreamExt;

use crate::context::{GlobalContext, ScopedContext};

/// Backoff between driver retries when the DB is unreachable.
const DB_RETRY: std::time::Duration = std::time::Duration::from_secs(15);

/// The cheap-clone scheduler handle held on
/// [`crate::context::ResidentHubs`]. `notify()` is the whole API —
/// the driver rebuilds everything else from the DB.
#[derive(Clone)]
pub struct TaskScheduler {
    notify: Arc<tokio::sync::Notify>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Wake the driver: the schedule changed (create / delete /
    /// run completion).
    pub fn notify(&self) {
        self.notify.notify_one();
    }

    /// Spawn the resident driver. Called once at daemon boot
    /// (`execute_foreground`), alongside the other resident tasks.
    pub fn spawn_driver(&self, global: GlobalContext, scoped: ScopedContext) {
        let notify = Arc::clone(&self.notify);
        let handle = self.clone();
        tokio::spawn(async move {
            // Boot reconcile — crashed in-flight runs re-arm, due now.
            // Retried, not fatal: the db may still be cold.
            loop {
                match global.db_client().await {
                    Ok(pool) => {
                        let now = chrono::Utc::now().timestamp();
                        if crate::db::tasks::reconcile_running(&pool, now).await.is_ok() {
                            break;
                        }
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(DB_RETRY).await;
            }
            loop {
                // One scan: when is the next fire due?
                let next = match global.db_client().await {
                    Ok(pool) => match crate::db::tasks::next_due(&pool).await {
                        Ok(next) => next,
                        Err(_) => {
                            tokio::time::sleep(DB_RETRY).await;
                            continue;
                        }
                    },
                    Err(_) => {
                        tokio::time::sleep(DB_RETRY).await;
                        continue;
                    }
                };
                match next {
                    None => {
                        // Nothing scheduled — park until the schedule
                        // changes.
                        notify.notified().await;
                    }
                    Some(due_at) => {
                        let now = chrono::Utc::now().timestamp();
                        if due_at > now {
                            let wait =
                                std::time::Duration::from_secs((due_at - now) as u64);
                            tokio::select! {
                                _ = notify.notified() => continue,
                                _ = tokio::time::sleep(wait) => {}
                            }
                        }
                        // Claim + fire everything due. Claim failures
                        // (db hiccup) just re-scan after backoff.
                        let now = chrono::Utc::now().timestamp();
                        let claimed = match global.db_client().await {
                            Ok(pool) => {
                                match crate::db::tasks::claim_due(&pool, now).await {
                                    Ok(claimed) => claimed,
                                    Err(_) => {
                                        tokio::time::sleep(DB_RETRY).await;
                                        continue;
                                    }
                                }
                            }
                            Err(_) => {
                                tokio::time::sleep(DB_RETRY).await;
                                continue;
                            }
                        };
                        for task in claimed {
                            fire(
                                handle.clone(),
                                global.clone(),
                                scoped.clone(),
                                task,
                            );
                        }
                    }
                }
            }
        });
    }
}

/// Run one claimed task to completion on its own spawned task: rebuild
/// the stored identity, feed the stored command through the
/// `--request` front door, drain the stream, record the outcome
/// (errored iff the LAST item was an error), and wake the driver.
fn fire(
    scheduler: TaskScheduler,
    global: GlobalContext,
    scoped: ScopedContext,
    task: crate::db::tasks::ClaimedTask,
) {
    tokio::spawn(async move {
        let stored = &task.agent_arguments;
        let mut run_scope = scoped
            .for_request(crate::context::ScopeIdentity::from_agent_arguments(stored))
            .await;
        // Restore the plugin trio — stamped as a set; from_agent_arguments
        // deliberately zeroed it (wire-unspoofability), the scheduler is
        // an AUTHORITY like `plugins run`.
        if let (Some(owner), Some(repository), Some(version)) = (
            stored.plugin_owner.as_deref(),
            stored.plugin_name.as_deref(),
            stored.plugin_version.as_deref(),
        ) {
            run_scope = run_scope.with_plugin(owner, repository, version);
        }
        // THE task-flag stamp — the only setter in the codebase.
        let run_scope = run_scope.with_task();

        // A run is ERRORED iff its LAST stream item is an error. A
        // stored command that no longer parses (or fails to serialize)
        // counts as an errored run too.
        let errored = match serde_json::to_string(&task.command) {
            Ok(json) => {
                let args =
                    vec!["objectiveai".to_string(), "--request".to_string(), json];
                match crate::run(args, Some((global.clone(), run_scope))).await {
                    Ok(crate::RunStream::Execute(mut stream)) => {
                        let mut last_errored = false;
                        while let Some(item) = stream.next().await {
                            last_errored = item.is_err();
                        }
                        last_errored
                    }
                    Ok(crate::RunStream::ExecuteTransform(mut stream)) => {
                        let mut last_errored = false;
                        while let Some(item) = stream.next().await {
                            last_errored = item.is_err();
                        }
                        last_errored
                    }
                    Err(_) => true,
                }
            }
            Err(_) => true,
        };

        // Completion UPDATE — targets the task by its sole identity
        // (`id` within its plugin namespace). Matches nothing when the
        // task was deleted mid-run (by design); errors are swallowed
        // (the boot reconcile re-arms a stranded `running` row).
        if let Ok(pool) = global.db_client().await {
            let now = chrono::Utc::now().timestamp();
            let plugin = (
                task.agent_arguments.plugin_owner.as_deref(),
                task.agent_arguments.plugin_name.as_deref(),
                task.agent_arguments.plugin_version.as_deref(),
            );
            let _ = crate::db::tasks::complete_run(
                &pool, plugin, &task.id, errored, now,
            )
            .await;
        }
        scheduler.notify();
    });
}

//! `kill-all` — terminate every process holding a lock anywhere under
//! the configured `OBJECTIVEAI_DIR`, EXCEPT the daemon running this
//! handler.
//!
//! The blunt counterpart to the targeted `{api,db,mcp,viewer} kill`:
//! rather than resolving one server by its known lock key, this asks
//! [`objectiveai_sdk::lockfile::owners_in_tree`] for every process
//! holding any `*.lock` anywhere in the dir tree and kills the lot —
//! the api server, per-state db supervisors (which take their
//! postmasters with them), viewers, and mcp servers.
//!
//! It runs INSIDE the daemon (via `/execute`), and deliberately skips
//! its OWN pid AND every DESCENDANT of it — not just because a
//! self-`TerminateProcess` would drop the /execute stream before the
//! response could be sent: the daemon's resident plugins are LEASHED
//! children (any plugin exiting ends the whole daemon — see `daemon
//! spawn`), and a resident plugin can hold locks under the tree (e.g.
//! a plugin-local `locks/mcp.lock`). Sweeping such a child killed the
//! daemon mid-response every time. So the sweep spares the daemon's
//! whole process tree and SURVIVES; the thin CLI then reaps that
//! EXACT tree — the daemon and every descendant, leashed or detached
//! — right after this response returns cleanly. (Killing only the
//! daemon pid was a bug: the laboratory HOST is a detached descendant
//! that survived every kill-all holding the dead daemon's address,
//! wedging every later create for the full host-connect timeout.) The
//! reported count is the OTHERS killed; the CLI adds its tree reap in.
//!
//! Two sweeps: killing a lock owner can orphan children that only
//! surface as lock owners once their parent is gone (a hard-killed
//! supervisor's plugin servers), and on Windows a dead process's lock
//! handle is released asynchronously — so the first pass exposes
//! stragglers the second pass then catches. PIDs actually terminated
//! are de-duplicated across both sweeps, so the reported count is
//! distinct processes killed. Idempotent: a count of zero is not an
//! error.

use std::collections::HashSet;

use objectiveai_sdk::cli::command::kill_all::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let dir = ctx.filesystem.dir().clone();
    let me = std::process::id();
    let mut killed: HashSet<u32> = HashSet::new();
    for _ in 0..2 {
        let pids = objectiveai_sdk::lockfile::owners_in_tree(&dir)
            .await
            .map_err(|e| Error::Spawn("read lock owners in tree".to_string(), e))?;
        // One process-table snapshot per pass — ancestry is checked
        // against it for every candidate.
        let mut sys = sysinfo::System::new();
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing(),
        );
        for pid in pids {
            // Never signal ourselves OR our descendants — the daemon
            // holds the `plugins-daemon` lock (killing it here would
            // truncate this very response; the CLI kills the daemon
            // after it returns), and its leashed resident plugins are
            // lock-holding children whose death ENDS the daemon. Also
            // skip the kernel's pid 0.
            if pid == me || pid == 0 || in_process_tree(&sys, pid, me) {
                continue;
            }
            if crate::spawn::kill_pid(pid) == 1 {
                killed.insert(pid);
            }
        }
    }
    Ok(Response { killed: killed.len() })
}

/// Whether `ancestor` appears in `pid`'s parent chain (per the given
/// process-table snapshot). Hop-capped: pid reuse can make parent
/// chains degenerate, and a missed skip only risks the daemon's own
/// life — err on bounded work, the chain is normally 2-3 hops.
fn in_process_tree(sys: &sysinfo::System, pid: u32, ancestor: u32) -> bool {
    let mut current = sysinfo::Pid::from_u32(pid);
    for _ in 0..64 {
        let Some(process) = sys.process(current) else {
            return false;
        };
        let Some(parent) = process.parent() else {
            return false;
        };
        if parent.as_u32() == ancestor {
            return true;
        }
        current = parent;
    }
    false
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::kill_all as sdk;
    use objectiveai_sdk::cli::command::kill_all::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::kill_all as sdk;
    use objectiveai_sdk::cli::command::kill_all::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}

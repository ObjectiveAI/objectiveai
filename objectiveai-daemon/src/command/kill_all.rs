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
//! its OWN pid — a self-`TerminateProcess` would drop the /execute stream
//! before the response could be sent. So the daemon sweeps everything
//! else and SURVIVES; the thin CLI kills the daemon itself right after
//! this response returns. The reported count is the OTHERS killed; the
//! CLI adds the daemon back in.
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
        for pid in pids {
            // Never signal ourselves — the daemon running this handler
            // holds the `plugins-daemon` lock, and killing it here would
            // truncate this very response (the CLI kills the daemon after
            // it returns). Also skip the kernel's pid 0.
            if pid == me || pid == 0 {
                continue;
            }
            if crate::spawn::kill_pid(pid) == 1 {
                killed.insert(pid);
            }
        }
    }
    Ok(Response { killed: killed.len() })
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

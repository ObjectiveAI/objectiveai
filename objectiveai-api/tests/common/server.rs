//! Spawns the `objectiveai-api` binary as a child process for the
//! duration of the integration test binary, and exposes an
//! [`objectiveai_sdk::http::HttpClient`] pointing at it.
//!
//! One server per integration test binary (cargo runs each
//! `tests/*.rs` as its own binary) → one ephemeral-port pool per
//! binary, one MCP proxy listener per binary. No cross-binary
//! interference at the OS level.
//!
//! Lifecycle:
//! - `client()` is the entry point. First call kicks the [`LazyLock`]
//!   below, which builds the `Command`, spawns the child, parses the
//!   bound address from the `listening on …` line on stderr, and
//!   constructs the HTTP client.
//! - `Drop for ServerHandle` kills the child unconditionally. The
//!   `LazyLock<ServerHandle>` only drops at process exit, so the
//!   server stays alive for the whole test binary.

use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use objectiveai_sdk::HttpClient;

mod kill_on_parent_exit {
    //! Tie the spawned server's lifetime to the test binary's. The
    //! `Drop` impl on `ServerHandle` is unreliable because Rust does
    //! not run destructors for `LazyLock` statics at process exit; we
    //! need OS-level lifetime tying so the child dies even on panic
    //! or `std::process::exit`.

    #[cfg(windows)]
    pub fn tie_to_parent(child: &std::process::Child) {
        use std::os::windows::io::AsRawHandle;
        use std::sync::OnceLock;
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW,
            JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        // Keep the job handle alive in a static so it's only released
        // when the test binary exits. At that point the kernel closes
        // the last handle, the job's `KILL_ON_JOB_CLOSE` flag fires,
        // and every assigned process (our spawned server) is killed.
        // Wrapping the handle in a transparent struct so the
        // !Send/!Sync raw pointer/HANDLE type lives in a static
        // safely (HANDLE is `*mut core::ffi::c_void`).
        struct SendHandle(HANDLE);
        unsafe impl Send for SendHandle {}
        unsafe impl Sync for SendHandle {}

        static JOB: OnceLock<SendHandle> = OnceLock::new();

        unsafe {
            let job = JOB
                .get_or_init(|| {
                    let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                    assert!(!h.is_null(), "CreateJobObjectW returned null");
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION =
                        std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    let ok = SetInformationJobObject(
                        h,
                        JobObjectExtendedLimitInformation,
                        &info as *const _ as *const _,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                    assert!(
                        ok != 0,
                        "SetInformationJobObject(KILL_ON_JOB_CLOSE) failed"
                    );
                    SendHandle(h)
                })
                .0;
            let ok = AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE);
            assert!(ok != 0, "AssignProcessToJobObject failed");
        }
    }

    #[cfg(unix)]
    pub fn tie_to_parent(_child: &std::process::Child) {
        // Wired up via `pre_exec` on the Command builder before spawn,
        // so this is a no-op after-the-fact. Kept symmetric with the
        // Windows path for future cross-platform changes.
    }
}

/// Per-test-binary handle to the spawned api server's bound URL.
/// `client()` builds a fresh `HttpClient` (and its reqwest pool) per
/// call so each `#[tokio::test]` runtime owns its own hyper dispatch
/// tasks — sharing a single reqwest pool across multiple per-test
/// runtimes panics with `runtime dropped the dispatch task` once the
/// first runtime drops its tasks.
pub struct ServerHandle {
    pub base_url: String,
    child: Mutex<Option<Child>>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

static SERVER: LazyLock<ServerHandle> = LazyLock::new(spawn_server);

/// Returns a fresh `HttpClient` pointing at the spawned api server.
/// Boots the server on first call. The returned client owns its own
/// reqwest pool — the calling test's runtime is the one that hosts
/// hyper's dispatch tasks, so when the test runtime drops at end of
/// test, those tasks shut down with it. Sharing one client across
/// multiple per-test runtimes deadlocks once the first runtime drops
/// its tasks.
pub fn client() -> Arc<HttpClient> {
    let base_url = SERVER.base_url.clone();
    Arc::new(HttpClient::new(
        reqwest::Client::new(),
        Some(base_url),
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
        None,
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
    ))
}

/// The spawned server's base URL, e.g. `http://127.0.0.1:53241`.
pub fn base_url() -> &'static str {
    &SERVER.base_url
}

/// Kill the spawned server child if any is alive. Idempotent — safe
/// to call from multiple cleanup paths.
fn shutdown() {
    if let Ok(mut guard) = SERVER.child.lock() {
        if let Some(mut c) = guard.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Atexit handler installed once during server spawn. The C runtime
/// invokes registered atexit handlers from `process::exit` (whether
/// libtest exits cleanly with all-passed, exits non-zero with
/// failures, or `process::exit`s after a top-level panic). Test
/// panics caught by libtest don't reach here — but they don't kill
/// the server either, since libtest catches them and continues. The
/// only way a panic skips this hook is `process::abort()`, which
/// terminates without atexit; the Windows Job Object backstop covers
/// that case.
unsafe extern "C" fn atexit_shutdown() {
    shutdown();
}

unsafe extern "C" {
    fn atexit(f: unsafe extern "C" fn()) -> i32;
}

fn spawn_server() -> ServerHandle {
    let bin = env!("CARGO_BIN_EXE_objectiveai-api");

    let mut cmd = Command::new(bin);
    cmd.env("ADDRESS", "127.0.0.1")
        .env("PORT", "0")
        // Disable the subprocess upstreams — tests only ever drive the
        // mock upstream, but Claude/Codex SDK clients try to find a
        // node binary at startup if enabled.
        .env("CLAUDE_AGENT_SDK_ENABLED", "false")
        .env("CODEX_SDK_ENABLED", "false")
        .env("MOCK_DELAY_MS", "0")
        .env("MOCK_MAX_TOOL_CALLS", "1000")
        // Use the in-process mock laboratory orchestrator so the server
        // doesn't need a real Docker daemon; the spawned api server's
        // `run.rs` swaps in `crate::laboratories::orchestrator::mock`
        // when this env var is set.
        .env("LABORATORY_USE_MOCK_ORCHESTRATOR", "1")
        // Generous wait limits so we don't time out on slow CI but with
        // zero retry/backoff so a real first-try MCP failure surfaces
        // instead of being masked. Each test binary has its own server
        // process, so the WSAEADDRINUSE pressure that motivated the
        // old backoff is gone.
        .env("MCP_CONNECT_TIMEOUT", "1800000")
        .env("MCP_CALL_TIMEOUT", "1800000")
        .env("MCP_BACKOFF_CURRENT_INTERVAL", "0")
        .env("MCP_BACKOFF_INITIAL_INTERVAL", "0")
        .env("MCP_BACKOFF_RANDOMIZATION_FACTOR", "0")
        .env("MCP_BACKOFF_MULTIPLIER", "1")
        .env("MCP_BACKOFF_MAX_INTERVAL", "0")
        .env("MCP_BACKOFF_MAX_ELAPSED_TIME", "0")
        .env("AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT", "1800000")
        .env("AGENT_COMPLETIONS_OTHER_CHUNK_TIMEOUT", "1800000")
        .env("AGENT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL", "0")
        .env("AGENT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL", "0")
        .env("AGENT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR", "0")
        .env("AGENT_COMPLETIONS_BACKOFF_MULTIPLIER", "1")
        .env("AGENT_COMPLETIONS_BACKOFF_MAX_INTERVAL", "0")
        .env("AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME", "0")
        // Bump invention tool-subscription wait from the 30s default to
        // 60s so contention-induced flakes (e.g. listener task pile-up
        // during parallel-suite load) don't surface as
        // `tool_subscription_timeout` errors.
        .env("FUNCTIONS_INVENTIONS_SUBSCRIBE_TOOLS_TIMEOUT", "60000")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("spawn objectiveai-api binary");
    kill_on_parent_exit::tie_to_parent(&child);
    // Register atexit hook so the server gets killed on every normal
    // exit path (libtest reports passes / failures and calls
    // `process::exit`, which runs atexit handlers).
    unsafe {
        atexit(atexit_shutdown);
    }
    let stderr = child.stderr.take().expect("piped stderr");

    // Read the server's stderr until we see the `listening on <addr>`
    // line emitted by `run.rs:1291`. Surface anything that looks like a
    // panic so failures are diagnosable.
    let addr = parse_listening_line(stderr).unwrap_or_else(|e| {
        let _ = child.kill();
        let _ = child.wait();
        panic!("failed to read api server's listening address from stderr: {e}");
    });

    let base_url = format!("http://{addr}");

    ServerHandle {
        base_url,
        child: Mutex::new(Some(child)),
    }
}

/// Reads the server's stderr line-by-line until it sees
/// `listening on <addr>`. The matched line is parsed as a
/// [`SocketAddr`]. Lines that don't match the prefix are ignored
/// (subsequent server logs flow into the void after this returns; we
/// don't currently capture them, but a future enhancement could keep
/// piping them to test stderr).
fn parse_listening_line(stderr: std::process::ChildStderr) -> Result<SocketAddr, String> {
    let started = std::time::Instant::now();
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("api server closed stderr before listening".into());
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("listening on ") {
            return rest
                .parse::<SocketAddr>()
                .map_err(|e| format!("malformed listening line {trimmed:?}: {e}"));
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err(format!(
                "api server did not announce listening within 60s; last line: {trimmed:?}"
            ));
        }
    }
}

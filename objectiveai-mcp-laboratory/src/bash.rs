const MAX_OUTPUT_CHARS: usize = 30_000;
const MAX_PERSISTED_SIZE: usize = 64 * 1024 * 1024; // 64MB

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dashmap::DashMap;
use tokio::fs;

#[derive(Debug, serde::Serialize)]
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub interrupted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "isImage", skip_serializing_if = "std::ops::Not::not")]
    pub is_image: bool,
    #[serde(skip_serializing_if = "Option::is_none", rename = "persistedOutputPath")]
    pub persisted_output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "persistedOutputSize")]
    pub persisted_output_size: Option<u64>,
    #[serde(rename = "noOutputExpected", skip_serializing_if = "std::ops::Not::not")]
    pub no_output_expected: bool,
}

/// Shell state shared by every agent dialing this laboratory MCP. CWD and
/// exported env are kept **per agent-instance-hierarchy (AIH)** so concurrent
/// agents sharing the one container don't trample each other's working
/// directory or environment; the snapshot, tmux socket, and shell path are
/// process-global (the snapshot is the container's static functions/aliases).
#[derive(Debug, Clone)]
pub struct ShellState {
    /// Per-AIH working directory, persisted across that AIH's commands.
    cwds: Arc<DashMap<String, PathBuf>>,
    /// Path to the shell environment snapshot file (functions, aliases, options).
    /// Captured once at session start from the user's shell config.
    snapshot_path: Arc<RwLock<Option<String>>>,
    /// Tmux socket env override. Set once tmux is first used.
    tmux_env: Arc<RwLock<Option<String>>>,
    /// Whether tmux has been used this session.
    tmux_used: Arc<RwLock<bool>>,
    /// The user's shell path (e.g., /bin/bash, /bin/zsh).
    shell_path: String,
    /// Working directory an AIH starts in before it runs anything (env
    /// `OBJECTIVEAI_LABORATORY_CWD`, baked in at `laboratories create`,
    /// defaulting to `/`). Only the first-touch fallback — once an AIH runs a
    /// command its cwd is tracked per-AIH in `cwds`.
    default_cwd: PathBuf,
}

impl ShellState {
    pub fn new(default_cwd: PathBuf) -> Self {
        let shell_path = detect_shell();
        Self {
            cwds: Arc::new(DashMap::new()),
            snapshot_path: Arc::new(RwLock::new(None)),
            tmux_env: Arc::new(RwLock::new(None)),
            tmux_used: Arc::new(RwLock::new(false)),
            shell_path,
            default_cwd,
        }
    }

    /// Initialize the shell snapshot asynchronously.
    /// Should be called once at session start.
    pub async fn init_snapshot(&self) {
        match create_shell_snapshot(&self.shell_path).await {
            Ok(path) => {
                *self.snapshot_path.write().unwrap() = Some(path);
            }
            Err(e) => {
                tracing::warn!("Failed to create shell snapshot: {e}");
            }
        }
    }

    /// This AIH's current working directory, or the laboratory's configured
    /// default the first time the AIH is seen.
    fn get_cwd(&self, aih: &str) -> PathBuf {
        self.cwds
            .get(aih)
            .map(|c| c.clone())
            .unwrap_or_else(|| self.default_cwd.clone())
    }

    fn set_cwd(&self, aih: &str, path: PathBuf) {
        self.cwds.insert(aih.to_string(), path);
    }

    fn get_snapshot_path(&self) -> Option<String> {
        self.snapshot_path.read().unwrap().clone()
    }

    fn mark_tmux_used(&self) {
        *self.tmux_used.write().unwrap() = true;
    }

    fn has_tmux_been_used(&self) -> bool {
        *self.tmux_used.read().unwrap()
    }

    fn get_tmux_env(&self) -> Option<String> {
        self.tmux_env.read().unwrap().clone()
    }

    fn set_tmux_env(&self, value: String) {
        *self.tmux_env.write().unwrap() = Some(value);
    }
}

pub async fn execute_bash(
    shell_state: &ShellState,
    aih: &str,
    command: &str,
    timeout_ms: Option<u64>,
) -> Result<BashOutput, String> {
    // Default 30 minutes, no hard max
    let timeout_ms = timeout_ms.unwrap_or(1_800_000);
    let timeout_duration = Duration::from_millis(timeout_ms);

    // Track tmux usage
    if command.contains("tmux") {
        shell_state.mark_tmux_used();
        if shell_state.get_tmux_env().is_none() {
            if let Ok(socket_path) = init_tmux_socket().await {
                shell_state.set_tmux_env(socket_path);
            }
        }
    }

    // Build the full command string with all session state. cwd + the exported
    // env file are per-AIH; the snapshot is process-global.
    let cwd = shell_state.get_cwd(aih);
    let env_path = env_path_for(aih);
    let snapshot_path = shell_state.get_snapshot_path();
    let has_snapshot = snapshot_path.is_some();

    let mut command_parts: Vec<String> = Vec::new();

    // 1. Source the shell snapshot (if available)
    if let Some(ref snap) = snapshot_path {
        command_parts.push(format!("source {} 2>/dev/null || true", shell_quote(snap)));
    }

    // 1b. Replay the exported env persisted after this AIH's previous command.
    // The file won't exist on the first call — `|| true` shrugs that off.
    command_parts.push(format!(
        "source {} 2>/dev/null || true",
        shell_quote(&env_path)
    ));

    // 3. Source CLAUDE_ENV_FILE if set (for venv/conda activation, parent process env persistence)
    if let Ok(env_file) = std::env::var("CLAUDE_ENV_FILE") {
        if !env_file.is_empty() && std::path::Path::new(&env_file).exists() {
            command_parts.push(format!("source {} 2>/dev/null || true", shell_quote(&env_file)));
        }
    }

    // 4. Disable extended glob patterns (security hardening)
    if std::env::var("CLAUDE_CODE_SHELL_PREFIX").is_ok() {
        // When using a shell wrapper, disable extglob for both bash and zsh
        command_parts.push("{ shopt -u extglob || setopt NO_EXTENDED_GLOB; } >/dev/null 2>&1 || true".to_string());
    } else if shell_state.shell_path.contains("bash") {
        command_parts.push("shopt -u extglob 2>/dev/null || true".to_string());
    } else if shell_state.shell_path.contains("zsh") {
        command_parts.push("setopt NO_EXTENDED_GLOB 2>/dev/null || true".to_string());
    }

    // 5. The user's command (wrapped in eval for alias expansion)
    command_parts.push(format!("eval {}", shell_quote(command)));

    // 6. After the command, capture BOTH cwd and exported env in one trailing
    // step: cwd → a per-call temp file (read back into ShellState below), env →
    // the persistent per-session file (sourced by the next command at step 1b).
    let cwd_file = cwd_file_path();
    command_parts.push(format!(
        "{{ pwd -P >| {}; export -p >| {}; }}",
        shell_quote(&cwd_file),
        shell_quote(&env_path),
    ));

    let command_string = command_parts.join(" && ");

    // Build spawn args: -c [-l] <command>
    // Use login shell (-l) only when no snapshot is available
    let mut args = vec!["-c".to_string()];
    if !has_snapshot {
        args.push("-l".to_string());
    }
    args.push(command_string);

    // Build environment overrides
    let mut env_overrides: HashMap<String, String> = HashMap::new();

    // Set SHELL to the detected shell path
    env_overrides.insert("SHELL".into(), shell_state.shell_path.clone());

    // Prevent git from opening an interactive editor (would hang)
    env_overrides.entry("GIT_EDITOR".into()).or_insert_with(|| "true".into());

    // Tmux socket isolation
    if let Some(tmux_env) = shell_state.get_tmux_env() {
        env_overrides.insert("TMUX".into(), tmux_env);
    }


    // Create a temp file to capture interleaved stdout+stderr at the OS level
    let output_file_path = tool_results_dir().join(format!(
        "bash-output-{}",
        CWD_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(tool_results_dir())
        .await
        .map_err(|e| format!("Failed to create tool-results dir: {e}"))?;

    let output_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&output_file_path)
        .await
        .map_err(|e| format!("Failed to create output file: {e}"))?
        .into_std()
        .await;

    let stdout_file = output_file
        .try_clone()
        .map_err(|e| format!("Failed to clone output file: {e}"))?;

    let mut cmd = tokio::process::Command::new(&shell_state.shell_path);
    cmd.args(&args)
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(output_file));
    // Give this exec its OWN process group (pgid == child pid), so
    // every descendant it spawns shares that group — the handle the
    // attribution engine maps back to this AIH. Inherited across fork;
    // unchanged for the child's own signal handling. Linux-only (the
    // laboratory always runs in a Linux container; attribution is
    // Linux-only too).
    #[cfg(target_os = "linux")]
    cmd.process_group(0);

    // Apply session env var overrides
    for (key, value) in &env_overrides {
        cmd.env(key, value);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {e}"))?;

    // Register this exec's process group under the caller's AIH for the
    // exec's lifetime, so filesystem writes it (and its descendants)
    // make are attributed to this agent. `child.id()` == the new pgid.
    let pgid = child.id().map(|p| p as i32);
    if let Some(pgid) = pgid {
        crate::attribution::register_session(pgid, aih);
    }

    // Wait for child to exit (output is in the file, not pipes)
    let (exit_code, interrupted) = match tokio::time::timeout(timeout_duration, child.wait()).await
    {
        Ok(Ok(status)) => (status.code(), false),
        Ok(Err(e)) => {
            if let Some(pgid) = pgid {
                crate::attribution::unregister_session(pgid);
            }
            return Err(format!("Command failed: {e}"));
        }
        Err(_) => {
            // Timeout — kill the child, then read whatever output was written
            let _ = child.kill().await;
            (None, true)
        }
    };
    if let Some(pgid) = pgid {
        crate::attribution::unregister_session(pgid);
    }

    // Read the output file
    let full_output = fs::read_to_string(&output_file_path).await.unwrap_or_default();
    let full_output_size = full_output.len();

    let mut persisted_output_path: Option<String> = None;
    let mut persisted_output_size: Option<u64> = None;

    let combined = if full_output_size > MAX_OUTPUT_CHARS {
        persisted_output_size = Some(full_output_size as u64);

        // The file is already on disk — truncate it if it exceeds MAX_PERSISTED_SIZE
        if full_output_size > MAX_PERSISTED_SIZE {
            let _ = fs::write(&output_file_path, &full_output[..MAX_PERSISTED_SIZE]).await;
        }
        persisted_output_path = Some(output_file_path.to_string_lossy().into_owned());

        // Build truncated inline output
        let total_lines = full_output.lines().count();
        let truncated = &full_output[..MAX_OUTPUT_CHARS];
        let kept_lines = truncated.lines().count();
        let dropped = total_lines - kept_lines;
        format!(
            "{}\n\n... [{} lines truncated] ...\nFull output ({} bytes) saved to: {}\nUse the Read tool to access it.",
            truncated,
            dropped,
            full_output_size,
            persisted_output_path.as_ref().unwrap()
        )
    } else {
        // Small output — clean up the temp file
        let _ = fs::remove_file(&output_file_path).await;
        full_output
    };

    // Read the saved CWD from the temp file and persist it for this AIH.
    if let Ok(new_cwd) = fs::read_to_string(&cwd_file).await {
        let new_cwd = new_cwd.trim();
        if !new_cwd.is_empty() {
            shell_state.set_cwd(aih, PathBuf::from(new_cwd));
        }
    }
    // Clean up the CWD file
    let _ = fs::remove_file(&cwd_file).await;

    let return_code_interpretation = interpret_return_code(command, exit_code);
    let no_output_expected = is_silent_command(command);

    let is_image = detect_base64_image(&combined);

    let stderr_msg = if interrupted {
        format!("Command timed out after {timeout_ms}ms")
    } else {
        String::new()
    };

    Ok(BashOutput {
        stdout: combined,
        stderr: stderr_msg,
        interrupted,
        exit_code,
        return_code_interpretation,
        is_image,
        persisted_output_path,
        persisted_output_size,
        no_output_expected,
    })
}

/// Detect the user's shell from environment.
/// Checks CLAUDE_CODE_SHELL first, then SHELL, then tries common paths on Windows.
/// Always returns a full path (or at least a validated executable path).
fn detect_shell() -> String {
    // 1. Check CLAUDE_CODE_SHELL env var first
    if let Ok(shell) = std::env::var("CLAUDE_CODE_SHELL") {
        if shell.contains("bash") || shell.contains("zsh") {
            return shell;
        }
    }

    // 2. Check SHELL env var
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }

    // 3. On Windows (msys/git-bash), try common paths then `which`
    if cfg!(windows) {
        for candidate in &["/usr/bin/bash", "/bin/bash"] {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
        // Fall back to `which bash`
        if let Ok(output) = std::process::Command::new("which")
            .arg("bash")
            .output()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return path;
            }
        }
    }

    // 4. Probe common shell locations and use the first that exists. A bare
    //    `/bin/bash` fallback fails on images that put bash elsewhere (the
    //    Alpine `bash` image installs it at `/usr/local/bin/bash`, with no
    //    `/bin/bash`) or that ship only a POSIX shell. Prefer bash, then fall
    //    back to `/bin/sh` (always present in a laboratory image — the
    //    container entrypoint itself is `/bin/sh`).
    for candidate in &[
        "/bin/bash",
        "/usr/bin/bash",
        "/usr/local/bin/bash",
        "/bin/sh",
        "/usr/bin/sh",
    ] {
        if std::path::Path::new(candidate).exists() {
            return candidate.to_string();
        }
    }

    "/bin/bash".into()
}

/// Generate a unique CWD temp file path for this invocation.
/// Uses an atomic counter combined with PID for uniqueness within a process.
static CWD_COUNTER: AtomicU64 = AtomicU64::new(0);
fn cwd_file_path() -> String {
    let id = CWD_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}/objectiveai-mcp-{}-{}-cwd",
        std::env::temp_dir().to_string_lossy(),
        std::process::id(),
        id,
    )
}

/// Per-AIH exported-env file path. The AIH (which contains `/`) is hashed so
/// the filename is filesystem-safe and collision-free; `export -p` is dumped
/// here after each of this AIH's commands and sourced before the next, giving
/// env the same cross-command persistence as cwd, scoped per agent instance.
fn env_path_for(aih: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    aih.hash(&mut h);
    format!(
        "{}/objectiveai-mcp-{}-env-{:016x}.sh",
        std::env::temp_dir().to_string_lossy(),
        std::process::id(),
        h.finish(),
    )
}

fn tool_results_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "objectiveai-mcp-{}-tool-results",
        std::process::id()
    ))
}

/// Parsed data URI components.
pub struct ParsedDataUri {
    pub media_type: String,
    pub data: String,
}

/// Parse a data URI string into its media type and base64 payload.
pub fn parse_data_uri(s: &str) -> Option<ParsedDataUri> {
    let trimmed = s.trim();
    let rest = trimmed.strip_prefix("data:")?;
    let (media_type, after) = rest.split_once(';')?;
    let data = after.strip_prefix("base64,")?;
    if media_type.is_empty() || data.is_empty() {
        return None;
    }
    Some(ParsedDataUri {
        media_type: media_type.to_string(),
        data: data.to_string(),
    })
}

/// Check if content starts with a base64-encoded image data URI.
/// Matches the official regex: /^data:image\/[a-z0-9.+_-]+;base64,/i
fn detect_base64_image(output: &str) -> bool {
    let s = output.as_bytes();
    // Case-insensitive prefix check: "data:image/"
    if s.len() < 11 {
        return false;
    }
    if !s[..11].eq_ignore_ascii_case(b"data:image/") {
        return false;
    }
    // Media subtype: [a-z0-9.+_-]+ then ";base64,"
    let rest = &s[11..];
    let mut i = 0;
    while i < rest.len() {
        let b = rest[i];
        if b.is_ascii_alphanumeric() || b == b'.' || b == b'+' || b == b'_' || b == b'-' {
            i += 1;
        } else {
            break;
        }
    }
    if i == 0 {
        return false;
    }
    rest[i..].starts_with(b";base64,")
}

/// Command-specific return code interpretation matching Claude Code's commandSemantics.
fn interpret_return_code(command: &str, exit_code: Option<i32>) -> Option<String> {
    let code = exit_code?;
    if code == 0 {
        return None;
    }

    // Extract the base command (first word, ignoring env vars and prefixes)
    let base_cmd = command
        .split(&['|', '&', ';'][..])
        .next()
        .unwrap_or("")
        .split_whitespace()
        .find(|w| !w.contains('=')) // skip env var assignments like FOO=bar
        .unwrap_or("");

    match base_cmd {
        // grep/rg: 0=matches, 1=no matches, 2+=error
        "grep" | "rg" | "egrep" | "fgrep" => {
            if code == 1 {
                Some("No matches found".into())
            } else {
                Some(format!("exit_code:{code}"))
            }
        }
        // diff: 0=identical, 1=differences found, 2+=error
        "diff" => {
            if code == 1 {
                Some("Files differ".into())
            } else {
                Some(format!("exit_code:{code}"))
            }
        }
        // find: 0=success, 1=some dirs inaccessible, 2+=error
        "find" => {
            if code == 1 {
                Some("Some directories were inaccessible".into())
            } else {
                Some(format!("exit_code:{code}"))
            }
        }
        // test/[: 0=true, 1=false, 2+=error
        "test" | "[" => {
            if code == 1 {
                Some("Condition is false".into())
            } else {
                Some(format!("exit_code:{code}"))
            }
        }
        _ => Some(format!("exit_code:{code}")),
    }
}

/// Detect if a command is expected to produce no output (silent commands).
fn is_silent_command(command: &str) -> bool {
    const SILENT_COMMANDS: &[&str] = &[
        "mv", "cp", "rm", "mkdir", "rmdir", "chmod", "chown", "chgrp",
        "touch", "ln", "cd", "export", "unset", "wait",
    ];
    const NEUTRAL_COMMANDS: &[&str] = &["echo", "printf", "true", "false", ":"];

    // Split on shell operators and check each command segment
    let mut has_non_fallback = false;
    let mut last_was_or = false;

    for segment in command.split(&['|', '&', ';'][..]) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let base_cmd = segment.split_whitespace().next().unwrap_or("");

        // After || operator, neutral commands don't count
        if last_was_or && NEUTRAL_COMMANDS.contains(&base_cmd) {
            last_was_or = false;
            continue;
        }
        last_was_or = segment.ends_with('|'); // rough heuristic for ||

        has_non_fallback = true;
        if !SILENT_COMMANDS.contains(&base_cmd) {
            return false;
        }
    }
    has_non_fallback
}

/// Simple shell quoting for a single argument.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Create a shell snapshot by sourcing the user's shell config and capturing
/// functions, aliases, and options. Returns the path to the snapshot file.
async fn create_shell_snapshot(shell_path: &str) -> Result<String, String> {
    let shell_type = if shell_path.contains("zsh") {
        "zsh"
    } else if shell_path.contains("bash") {
        "bash"
    } else {
        "sh"
    };

    let config_file = get_config_file(shell_path);

    // Config file is optional — snapshot still captures Claude Code defaults
    let has_config = std::path::Path::new(&config_file).exists();

    let snapshot_path = format!(
        "{}/objectiveai-mcp-snapshot-{}-{}.sh",
        std::env::temp_dir().to_string_lossy(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    );

    let source_line = if has_config {
        format!("source {} 2>/dev/null", shell_quote(&config_file))
    } else {
        "true".into()
    };

    let is_windows = cfg!(windows);

    let snapshot_script = if shell_type == "zsh" {
        format!(
            r#"
SNAPSHOT_FILE={snapshot}
{source}
{{
  echo '# Shell snapshot (zsh)'
  typeset -f 2>/dev/null
  alias | sed 's/^alias //g' | sed 's/^/alias -- /'
  setopt 2>/dev/null | while IFS= read -r opt; do echo "setopt $opt"; done
}} > "$SNAPSHOT_FILE" 2>/dev/null
"#,
            snapshot = shell_quote(&snapshot_path),
            source = source_line,
        )
    } else {
        // On Windows (msys/git-bash), filter out winpty aliases
        let alias_cmd = if is_windows {
            r#"alias | grep -v "='winpty " | sed 's/^alias //g' | sed 's/^/alias -- /'"#
        } else {
            r#"alias | sed 's/^alias //g' | sed 's/^/alias -- /'"#
        };

        format!(
            r#"
SNAPSHOT_FILE={snapshot}
unalias -a 2>/dev/null || true
{source}
{{
  echo '# Shell snapshot (bash)'
  declare -f 2>/dev/null
  {alias_cmd}
  shopt -p 2>/dev/null
  set -o | grep "on" | awk '{{print "set -o " $1}}'
  echo "shopt -s expand_aliases"
}} > "$SNAPSHOT_FILE" 2>/dev/null
"#,
            snapshot = shell_quote(&snapshot_path),
            source = source_line,
            alias_cmd = alias_cmd,
        )
    };

    let result = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(shell_path)
            .args(["-c", &snapshot_script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| "Shell snapshot creation timed out".to_string())?
    .map_err(|e| format!("Failed to create snapshot: {e}"))?;

    if !std::path::Path::new(&snapshot_path).exists() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("Snapshot file was not created: {stderr}"));
    }

    tracing::info!("Shell snapshot created at {snapshot_path}");
    Ok(snapshot_path)
}

/// Get the shell config file path based on shell type.
fn get_config_file(shell_path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    if shell_path.contains("zsh") {
        format!("{home}/.zshrc")
    } else {
        format!("{home}/.bashrc")
    }
}

/// Initialize an isolated tmux socket for this session.
async fn init_tmux_socket() -> Result<String, String> {
    let socket_path = format!(
        "{}/objectiveai-mcp-tmux-{}.sock",
        std::env::temp_dir().to_string_lossy(),
        std::process::id(),
    );

    let output = tokio::process::Command::new("tmux")
        .args(["-S", &socket_path, "new-session", "-d", "-s", "objectiveai"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to initialize tmux socket: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux initialization failed: {stderr}"));
    }

    tracing::info!("Tmux socket initialized at {socket_path}");
    Ok(socket_path)
}

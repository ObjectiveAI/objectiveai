//! The one install: Claude Code, fetched at startup.
//!
//! The image ships WITHOUT Claude Code, for licensing reasons — the
//! harness installs it itself, the moment the process starts, and
//! everything that would touch `claude` waits on that one outcome:
//! the run refuses to spawn before it, and a failed install turns
//! every endpoint into the same error, forever. There is no retry —
//! the outcome is memoized and final; a container whose install
//! failed is replaced, not healed.

use tokio::process;
use tokio::sync::OnceCell;

/// The pinned package — the version whose flags the spawn argv
/// assumes. The pin lives HERE, nowhere else: the Containerfile
/// installs nothing.
const PACKAGE: &str = "@anthropic-ai/claude-code@2.1.246";

/// The install's memoized outcome.
static INSTALL: OnceCell<Result<(), String>> = OnceCell::const_new();

/// The install's outcome — starting the install if nobody has.
///
/// tokio's `OnceCell` runs ONE initializer and makes every
/// concurrent caller wait on it: `main` calls this from a spawned
/// task at startup (the immediate kick), and every endpoint awaits
/// it after — blocking while the install runs, then seeing the
/// memoized outcome forever.
pub async fn installed() -> &'static Result<(), String> {
    INSTALL.get_or_init(install).await
}

/// `npm install -g` of the pinned package, its failure rendered as
/// the prose it exists to be: the error is only ever reported.
async fn install() -> Result<(), String> {
    let output = process::Command::new("npm")
        .arg("install")
        .arg("-g")
        .arg(PACKAGE)
        .output()
        .await
        .map_err(|error| {
            format!("npm could not be spawned: {error}")
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "npm install failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

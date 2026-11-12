//! A command that answers what it has at once, and leaves its wait
//! behind.
//!
//! A device login is two things: a code, had in a moment, and a wait
//! for the code to be decided, which may be minutes. The command that
//! begins one prints the code and ends, so that whoever ran it — an
//! agent, most of all — has the code and its turn back at once; the
//! wait goes on without it. The rules:
//!
//! 1. **Two processes.** The command, run, is the **front**. It runs
//!    this same program again, with the same arguments and
//!    [`WAITER`] set in its environment: the **waiter**, which does
//!    the command's work. The front does none of it.
//! 2. **Apart.** The waiter is made apart from the front's console
//!    and its process group, with no stdin, and is not ended when the
//!    front is: neither the front ending nor its terminal closing
//!    takes it down.
//! 3. **Relayed until the waiter leaves.** The waiter's stdout is a
//!    pipe to the front, which prints each line of it as its own.
//!    Every line the command prints is JSON, so none is empty: an
//!    empty line is the waiter saying [`leave`], and the front ends
//!    there, successfully, having printed what came before.
//! 4. **A waiter that ends first** — refused before it had a code, or
//!    failed — is waited for, and its ending is the front's: nothing
//!    more where it succeeded, and [`Error::Failed`] with what it
//!    said where it did not.
//! 5. **After leaving, nobody listens.** The front's end of the pipe
//!    is gone, so the waiter prints nothing more, and how its wait
//!    ended is read where it left it: a login signed in is an
//!    identity among the identities, and one that was not is none.

use std::fmt;
use std::io;
use std::process::{ExitStatus, Stdio};

use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};

/// The environment variable that makes a run the waiter — see the
/// [rules](self), rule 1.
pub const WAITER: &str = "DIVERGE_WAITER";

/// Whether this run is the waiter.
pub fn waiter() -> bool {
    std::env::var_os(WAITER).is_some()
}

/// Why the front could not relay its waiter.
#[derive(Debug)]
pub enum Error {
    /// This program's own path could not be had.
    Binary(io::Error),
    /// The waiter could not be started.
    Spawn(io::Error),
    /// The waiter's lines could not be read.
    Read(io::Error),
    /// Stdout could not be written.
    Stdout(io::Error),
    /// The waiter's ending could not be had.
    Wait(io::Error),
    /// The waiter ended, unsuccessfully, before it left: how, and
    /// what it said of it.
    Failed {
        /// How it ended.
        status: ExitStatus,
        /// What it printed on stderr.
        said: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Binary(error) => write!(f, "this program's own path could not be had: {error}"),
            Error::Spawn(error) => write!(f, "the command could not be started apart: {error}"),
            Error::Read(error) => write!(f, "the command's answers could not be read: {error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
            Error::Wait(error) => write!(f, "the command's ending could not be had: {error}"),
            // The waiter's own report, less the `Error: ` its `main`
            // led it with, which this one's `main` leads with again.
            Error::Failed { status, said } => match said.trim().strip_prefix("Error: ").unwrap_or(said.trim()) {
                "" => write!(f, "the command ended, {status}, and said nothing"),
                said => f.write_str(said),
            },
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Binary(error) | Error::Spawn(error) | Error::Read(error) | Error::Stdout(error) | Error::Wait(error) => Some(error),
            Error::Failed { .. } => None,
        }
    }
}

/// The front's whole run — see the [rules](self): the waiter started
/// apart, its lines printed until it leaves or ends.
pub async fn relay() -> Result<(), Error> {
    let binary = std::env::current_exe().map_err(Error::Binary)?;
    // Built as the standard library's command, whose way of setting a
    // process apart is the same on every toolchain, and run as
    // tokio's.
    let mut apart = std::process::Command::new(binary);
    apart.args(std::env::args_os().skip(1)).env(WAITER, "1").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        apart.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        apart.process_group(0);
    }
    let mut command = tokio::process::Command::from(apart);
    command.kill_on_drop(false);
    let mut waiter = command.spawn().map_err(Error::Spawn)?;
    let mut stdout = tokio::io::stdout();
    if let Some(lines) = waiter.stdout.take() {
        let mut lines = BufReader::new(lines).lines();
        // Rule 3.
        while let Some(line) = lines.next_line().await.map_err(Error::Read)? {
            if line.is_empty() {
                return Ok(());
            }
            stdout.write_all(line.as_bytes()).await.map_err(Error::Stdout)?;
            stdout.write_all(b"\n").await.map_err(Error::Stdout)?;
            stdout.flush().await.map_err(Error::Stdout)?;
        }
    }
    // Rule 4.
    let status = waiter.wait().await.map_err(Error::Wait)?;
    if status.success() {
        return Ok(());
    }
    let mut said = String::new();
    if let Some(mut stderr) = waiter.stderr.take() {
        // What it said is for the report alone: unread, the report
        // says less, and the failure is the same.
        let _ = stderr.read_to_string(&mut said).await;
    }
    Err(Error::Failed { status, said })
}

/// The waiter leaving: the empty line that ends the front — see the
/// [rules](self), rule 3. A front already gone is a line nobody
/// reads, which changes nothing for the wait.
pub async fn leave() {
    let mut stdout = tokio::io::stdout();
    let _ = stdout.write_all(b"\n").await;
    let _ = stdout.flush().await;
}

/// The Windows process creation flag that gives the waiter no console
/// window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The Windows process creation flag that puts the waiter apart from
/// the front's console, so that the console closing does not end it.
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

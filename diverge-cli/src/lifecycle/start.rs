//! Starting a program on the root.

use std::path::Path;
use std::process::Stdio;

use diverge_sdk::file_lock;

use super::{Error, POLL, Program, beside, say};

/// `program` started on `root`, detached, and the command ended once
/// it holds the root's lock.
///
/// - The root's lock is probed first — [`file_lock::try_exclusive`],
///   and the probe let go before anything is spawned — and a root
///   already held is [`Error::Running`], with what the holder wrote
///   in the lock file, its process id, so that a second start is
///   refused here with a name rather than by the program with an
///   exit.
/// - The log file is made, truncating what an earlier run left, and
///   handed to the child as both its stdout and its stderr, the one
///   handle cloned; the child's stdin is nothing. The programs print
///   nothing on success and their failures on stderr, so the log is
///   where a death is explained.
/// - The child is the program's binary beside this one — see
///   [`beside`] — given `--config <root>`, the CLI's root, so that it
///   runs on the root this command found; `DIVERGE_CONFIG` and the
///   working directory, which the program would otherwise consult,
///   are not what it sees. It is not killed when this command ends,
///   and on Windows it is made with no console window and apart from
///   this one's, so that neither a closed terminal nor this process
///   ending takes it down.
/// - Then, every 200 ms, two things are asked, in this order. Has the
///   child exited? Then it died before it held the lock — a port
///   taken, a document it refused, a root it could not use — and that
///   is [`Error::Died`], with the whole of the log. Else, does the
///   lock file name the child's process id, and is the lock held? The
///   program writes its id only once it holds the lock, so the first
///   is the program having started and the second that it still has;
///   both true, the start is done and `started <word>, process <id>`
///   is printed. The programs announce nothing of their own, and a
///   pipe held open for one to write on would tie the detached child
///   to this command; the lock file is what they do leave, so it is
///   polled.
pub async fn start(root: &Path, program: Program) -> Result<(), Error> {
    let lock = program.lock(root);
    match file_lock::try_exclusive(&lock).await.map_err(Error::Lock)? {
        Some(probe) => drop(probe),
        None => {
            let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
            return Err(Error::Running { program, holder });
        }
    }
    let log = program.log(root);
    let file = tokio::fs::File::create(&log)
        .await
        .map_err(|source| Error::Io {
            path: log.clone(),
            source,
        })?;
    let stdout = file.into_std().await;
    let stderr = stdout.try_clone().map_err(|source| Error::Io {
        path: log.clone(),
        source,
    })?;
    let binary = beside(&program.binary())?;
    let mut command = tokio::process::Command::new(&binary);
    command
        .arg("--config")
        .arg(root)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .kill_on_drop(false);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    let mut child = command.spawn().map_err(|source| Error::Spawn {
        path: binary,
        source,
    })?;
    loop {
        if let Some(status) = child.try_wait().map_err(Error::Wait)? {
            let log = tokio::fs::read_to_string(&log).await.unwrap_or_default();
            return Err(Error::Died {
                program,
                status,
                log,
            });
        }
        let holder = file_lock::holder(&lock).await.map_err(Error::Lock)?;
        let named = holder.and_then(|holder| holder.parse::<u32>().ok());
        if named.is_some() && named == child.id() {
            let probe = file_lock::try_exclusive(&lock).await.map_err(Error::Lock)?;
            if probe.is_none() {
                let pid = named.unwrap_or_default();
                return say(&format!("started {}, process {pid}", program.word())).await;
            }
        }
        tokio::time::sleep(POLL).await;
    }
}

/// The Windows process creation flag that gives the child no console
/// window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The Windows process creation flag that puts the child apart from
/// this process's console, so that the console closing does not end
/// it.
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

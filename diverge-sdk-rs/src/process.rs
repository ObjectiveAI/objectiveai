//! How a Diverge program spawns another, so that nobody sees a window.
//!
//! The daemon, the provider and the Postgres supervisor are services:
//! started detached from any console, running until they are stopped,
//! showing nothing. Every program one of them runs — podman, `ssh`,
//! `postgres`, `initdb`, `pg_isready`, `pg_ctl`, a hook, the
//! supervisor itself — is a console program, and on Windows a console
//! program spawned by a process with no console is given a fresh one
//! of its own: a window, flashed at whoever is logged in, for every
//! invocation. [`no_window`] is the one rule that stops it, applied
//! by every runtime spawn site in the workspace's programs, so that a
//! reader auditing for the flash looks for one name.
//!
//! On Linux and macOS a process has no window to be given, and the
//! rule does nothing. It is gated inside, not at its call sites: a
//! spawn site is written once, the same on every host.

use tokio::process::Command;

/// Spawn `command`'s child with no console window of its own. On
/// Windows this sets the `CREATE_NO_WINDOW` process creation flag,
/// `0x08000000`, through tokio's own `creation_flags`, which is the
/// only way a console-subsystem child of a windowless parent is kept
/// from opening a console; it does not detach the child from the
/// parent, and it does not change the child's standard streams,
/// which stay as the command set them. Elsewhere it does nothing.
pub fn no_window(command: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

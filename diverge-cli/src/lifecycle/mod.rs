//! The `daemon` and `provider` commands: the program on this host
//! started, or stopped.
//!
//! `diverge daemon start|stop`, `diverge provider start|stop`: the
//! [`Program`] is the daemon's or the provider's own binary beside
//! this one, and the root is the CLI's, handed to the program as
//! `--config <root>` so that both run on the one root this command
//! found.
//!
//! - [`start`] spawns the program detached — its stdin nothing, its
//!   stdout and stderr the program's log in the root — and ends once
//!   the program holds the root's lock with its process id written
//!   in it, polling the lock file every 200 ms, since the programs
//!   announce nothing of their own. A root already held is refused
//!   with the holder's id; a program that exits first is reported
//!   with its log.
//! - [`stop`] puts `POST /stop` to the program's own port over a
//!   loopback socket — the one HTTP exchange the CLI makes, hand
//!   written over a TCP stream, see [`http::stop`], the one file
//!   here not flattened, since its `stop` is not [`stop`] — and ends
//!   once the root's lock is free, polling it every 200 ms: the
//!   server's close is the accept and the lock let go is the
//!   completion, the whole of the program's teardown. The daemon's
//!   port is the default daemon's in `cli.yaml`, which must be on
//!   this host; the provider's is `provider.yaml`'s, read as the
//!   provider reads it — see [`port`]. `daemon.yaml` is never
//!   consulted.
//!
//! Neither command takes an account or a credential: the route is
//! honoured by the program for any loopback peer, and the lock file
//! is the operating system's. [`Error`] is why either could not be.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

use std::path::Path;
use std::time::Duration;

use crate::arguments::Lifecycle;

mod beside;
mod error;
mod port;
mod program;
mod say;
mod start;
mod stop;

pub mod http;

pub use beside::*;
pub use error::*;
pub use port::*;
pub use program::*;
pub use say::*;
pub use start::*;
pub use stop::*;

/// How often the lock file is asked, by a start for the program to
/// hold it and by a stop for the program to let it go.
pub const POLL: Duration = Duration::from_millis(200);

/// Do `action` to `program` on `root`: see [`start`] and [`stop`].
pub async fn run(root: &Path, program: Program, action: Lifecycle) -> Result<(), Error> {
    match action {
        Lifecycle::Start => start(root, program).await,
        Lifecycle::Stop => stop(root, program).await,
    }
}

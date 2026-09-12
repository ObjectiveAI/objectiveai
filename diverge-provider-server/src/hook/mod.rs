//! Hooks: programs the operator supplies, run to answer a question.
//!
//! A hook is a FOLDER under the provider's `hooks/` directory, and
//! wherever configuration names one it names the folder and nothing
//! else. The convention, in full:
//!
//! - **The folder.** `hooks/<name>/`, holding everything the hook
//!   needs — its script, its environment, its data — and one file,
//!   [`hook.yaml`](MANIFEST), that says how to run it.
//! - **The manifest.** One argv array per platform, each optional, no
//!   shell, no default: the running platform's entry is the command,
//!   and a manifest without one is an error. The first element is the
//!   program; a relative path that exists in the folder is that file,
//!   anything else is passed to the OS as written. The rest are the
//!   arguments, verbatim; nothing is appended.
//! - **The working directory** is the hook's folder, so every relative
//!   path the command names means "in here", on every platform.
//! - **The input** is one JSON document, on one line, on stdin, and
//!   stdin is then closed.
//! - **The output** is exit `0` and stdout as one JSON document, read
//!   into the type the question expects. Any other exit is an error
//!   carrying the status and what the hook wrote to stdout and
//!   stderr; an exit `0` whose stdout is not the expected type is an
//!   error carrying the path of the mismatch.
//! - **No timeout.** A hook is waited for until it exits, and is
//!   killed only when the question is abandoned.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod manifest;
mod run;

pub use error::*;
pub use manifest::*;
pub use run::*;

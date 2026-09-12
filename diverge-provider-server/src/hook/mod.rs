//! A command the operator supplies, run to answer a question.
//!
//! Every program the provider asks something of is a [`Hook`]: an
//! argv array in `config.yaml`, no shell. The convention, in full:
//!
//! - **The program.** The first element. An absolute path is run as
//!   it is; a bare name or a relative path resolves first against the
//!   provider's `hooks/` directory, then on `PATH`. The process runs
//!   with `hooks/` as its working directory.
//! - **The question.** One JSON document on stdin, and the identity
//!   it concerns as `DIVERGE_PROVIDER_IDENTITY` in the environment —
//!   never in argv, which every user of the machine can read. Nothing
//!   is appended to argv: the elements after the first are the
//!   arguments, exactly as written.
//! - **The answer.** Exit `0`, and stdout as one JSON document read
//!   into the type the question expects; an empty stdout reads as
//!   `null`, so a hook with nothing to say answers `()` or `None`.
//!   Every other exit is the hook saying no, or failing, and either
//!   is an [`Error`] carrying the status and stderr. stderr is the
//!   operator's channel and is never parsed.
//! - **No timeout.** A hook is waited for until it exits.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod hook;
mod run;

pub use error::*;
pub use hook::*;
pub use run::*;

//! One turn of Python: the script run against the conversation, and
//! its output read back as chunks.
//!
//! [`run`] is the whole of it: the [`Feed`] serialized onto the
//! harness's stdin, the process waited for, the envelope taken from
//! its last stdout line ([`process`]), and the envelope's value
//! deserialized and checked ([`output()`]). No streaming — the script
//! answers once, when it ends — and no timeout: the script runs as
//! long as it runs.

mod envelope;
mod error;
mod feed;
mod output;
mod process;

pub use envelope::*;
pub use error::*;
pub use feed::*;
pub use output::*;
pub use process::*;

use diverge_provider_sdk::shared::containers::run_loop::response::AgenticLoopChunk;

/// Run the script once, and read what it answered.
pub async fn run(feed: &Feed<'_>) -> Result<Vec<AgenticLoopChunk>, Error> {
    let envelope = invoke(feed).await?;
    output(envelope)
}

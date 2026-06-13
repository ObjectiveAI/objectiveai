//! Shared 5-variant resolver used by `agents spawn`'s [`PromptSource`]
//! and `agents message`'s [`MessageSource`]. Each subcommand exposes
//! the same five long flags:
//!
//! - `--simple <text>`         — plain text the helper threads through
//!   `parse_simple` to build the target shape
//!   (`Vec<Message>` for prompt, `RichContent` for message).
//! - `--inline <JSON>`         — JSON literal of the target type.
//! - `--file <path>`           — JSON file holding the target type.
//! - `--python-inline <code>`  — inline Python that produces the
//!   target type as JSON.
//! - `--python-file <path>`    — Python file that produces the
//!   target type as JSON.
//!
//! The clap struct stays per-call (each subcommand declares its own
//! `Args`) because the doc strings + flag groups diverge slightly;
//! this helper only owns the dispatch logic so the wire/JSON/python
//! plumbing isn't duplicated.

use std::path::PathBuf;

use serde::de::DeserializeOwned;

/// Resolve one of the 5 source variants into a typed target.
///
/// Exactly one of the five `Option<…>` arguments is expected to be
/// `Some` — the caller's clap layer enforces that via
/// `#[group(required = true, multiple = false)]`. Panics on the
/// "all `None`" path with an `unreachable!()` rather than returning
/// an error, since the clap group makes that state impossible.
///
/// The python variants run on [`crate::context::Context::python`],
/// initialized lazily INSIDE those arms — non-python sources never
/// touch the wasm runtime.
pub async fn resolve_source<T, F>(
    ctx: &crate::context::Context,
    simple: Option<String>,
    inline: Option<String>,
    file: Option<PathBuf>,
    python_inline: Option<String>,
    python_file: Option<PathBuf>,
    parse_simple: F,
) -> Result<T, crate::error::Error>
where
    T: DeserializeOwned,
    F: FnOnce(String) -> T,
{
    if let Some(text) = simple {
        return Ok(parse_simple(text));
    }
    if let Some(inline) = inline {
        let mut de = serde_json::Deserializer::from_str(&inline);
        return serde_path_to_error::deserialize(&mut de)
            .map_err(crate::error::Error::InlineDeserialize);
    }
    if let Some(path) = file {
        let bytes = std::fs::read(&path)
            .map_err(|e| crate::error::Error::PromptFileRead(path.clone(), e))?;
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        return serde_path_to_error::deserialize(&mut de)
            .map_err(crate::error::Error::InlineDeserialize);
    }
    if let Some(code) = python_inline {
        return ctx
            .python()
            .await?
            .exec_code(&code, None::<()>)
            .await?
            .ok_or(crate::error::Error::PythonNoOutput);
    }
    if let Some(path) = python_file {
        return ctx
            .python()
            .await?
            .exec_file(&path, None::<()>)
            .await?
            .ok_or(crate::error::Error::PythonNoOutput);
    }
    unreachable!("clap group ensures one of the five flags is set")
}

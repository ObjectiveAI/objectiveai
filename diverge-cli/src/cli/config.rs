//! The `config` command beneath every program, done.

use std::path::Path;

use tokio::io::AsyncWriteExt as _;

use diverge_sdk::daemon::patch;

use super::Error;
use crate::arguments::Action;
use crate::document::{self, Which};
use crate::jq;

/// Do `action` to the document `which` names in the root — the
/// document of the program the command is beneath: a `get` prints every
/// value the filter yields — `.` when none was given — as JSON, one
/// line each; a `set` runs the program, takes
/// the one value it yields as the new document, validates it by the
/// document's type, and writes it; a `patch` applies the JSON Patch
/// to the document, whole or not at all, validates the result by the
/// document's type, and writes it.
pub async fn run(root: &Path, which: Which, action: Action) -> Result<(), Error> {
    match action {
        Action::Get { filter } => {
            let current = document::read(root, which).await?;
            let values =
                jq::run(filter.as_deref().unwrap_or("."), current).map_err(document::Error::Jq)?;
            let mut stdout = tokio::io::stdout();
            for value in values {
                let mut text = serde_json::to_vec(&value).map_err(document::Error::Json)?;
                text.push(b'\n');
                stdout
                    .write_all(&text)
                    .await
                    .map_err(document::Error::Stdout)?;
            }
            stdout.flush().await.map_err(document::Error::Stdout)?;
            Ok(())
        }
        Action::Set { filter } => set(root, which, &filter).await,
        Action::Patch { patch: text } => {
            let patch = document::patch(&text)?;
            let current = document::read(root, which).await?;
            let patched = patch::apply(current, &patch).map_err(document::Error::Apply)?;
            let yaml = which.validate(patched)?;
            document::write(root, which, &yaml).await?;
            Ok(())
        }
    }
}

/// The document rewritten by `program`: exactly one value yielded,
/// validated by the document's type, and written.
async fn set(root: &Path, which: Which, program: &str) -> Result<(), Error> {
    let current = document::read(root, which).await?;
    let mut values = jq::run(program, current).map_err(document::Error::Jq)?;
    if values.len() != 1 {
        return Err(document::Error::NotOne(values.len()).into());
    }
    let yaml = which.validate(values.remove(0))?;
    document::write(root, which, &yaml).await?;
    Ok(())
}

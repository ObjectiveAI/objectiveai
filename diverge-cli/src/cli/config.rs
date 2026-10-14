//! The `config` command, done.

use std::path::Path;

use tokio::io::AsyncWriteExt as _;

use super::Error;
use crate::arguments::{Action, Document};
use crate::document::{self, Which};
use crate::jq;

/// Do `action` to the document in the root: a `get` prints every
/// value the filter yields — `.` when none was given — as JSON, one
/// line each; a `set` runs the program, takes
/// the one value it yields as the new document, validates it by the
/// document's type, and writes it; an `edit` is a `set` of the program
/// `(SEL) = $value` — or `del(SEL)` — with the YAML value bound as
/// `$value`.
pub async fn run(root: &Path, document: Document) -> Result<(), Error> {
    let (which, action) = match document {
        Document::Daemon { action } => (Which::Daemon, action),
        Document::Provider { action } => (Which::Provider, action),
        Document::Cli { action } => (Which::Cli, action),
    };
    match action {
        Action::Get { filter } => {
            let current = document::read(root, which).await?;
            let values = jq::run(filter.as_deref().unwrap_or("."), current, None)
                .map_err(document::Error::Jq)?;
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
        Action::Set { filter } => set(root, which, &filter, None).await,
        Action::Edit {
            selector,
            value,
            delete,
        } => {
            let value = match value {
                Some(text) => Some(document::value(&text)?),
                None => None,
            };
            set(root, which, &document::program(&selector, delete), value).await
        }
    }
}

/// The document rewritten by `program`, with `value` bound as `$value`
/// when there is one: exactly one value yielded, validated by the
/// document's type, and written.
async fn set(
    root: &Path,
    which: Which,
    program: &str,
    value: Option<serde_json::Value>,
) -> Result<(), Error> {
    let current = document::read(root, which).await?;
    let mut values = jq::run(program, current, value).map_err(document::Error::Jq)?;
    if values.len() != 1 {
        return Err(document::Error::NotOne(values.len()).into());
    }
    let yaml = which.validate(values.remove(0))?;
    document::write(root, which, &yaml).await?;
    Ok(())
}

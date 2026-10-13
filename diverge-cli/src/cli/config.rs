//! The `config` command, done.

use std::path::Path;

use tokio::io::AsyncWriteExt as _;

use crate::arguments::{Action, Document};
use crate::document::{self, Which};
use super::Error;

/// Do `action` to the document in the root: a `get` prints every
/// value the filter yields — `.` when none was given — as JSON, one
/// line each; a `set` runs the program, takes
/// the one value it yields as the new document, validates it by the
/// document's type, and writes it; an `edit` opens the document in the
/// editor and writes what it left, validated the same way.
pub async fn run(root: &Path, document: Document) -> Result<(), Error> {
    let (which, action) = match document {
        Document::Daemon { action } => (Which::Daemon, action),
        Document::Provider { action } => (Which::Provider, action),
        Document::Cli { action } => (Which::Cli, action),
    };
    match action {
        Action::Get { filter } => {
            let current = document::read(root, which).await?;
            let values = document::run(filter.as_deref().unwrap_or("."), current)?;
            let mut stdout = tokio::io::stdout();
            for value in values {
                let mut text = serde_json::to_vec(&value).map_err(document::Error::Json)?;
                text.push(b'\n');
                stdout.write_all(&text).await.map_err(document::Error::Stdout)?;
            }
            stdout.flush().await.map_err(document::Error::Stdout)?;
            Ok(())
        }
        Action::Set { filter } => {
            let current = document::read(root, which).await?;
            let mut values = document::run(&filter, current)?;
            if values.len() != 1 {
                return Err(document::Error::NotOne(values.len()).into());
            }
            let yaml = which.validate(values.remove(0))?;
            document::write(root, which, &yaml).await?;
            Ok(())
        }
        Action::Edit => {
            let yaml = document::edit(root, which).await?;
            document::write(root, which, &yaml).await?;
            Ok(())
        }
    }
}

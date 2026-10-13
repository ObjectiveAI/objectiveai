//! Editing a document in the user's editor.

use std::path::Path;
use std::process::Stdio;

use super::{Error, Which, read};

/// The document, edited: its YAML as its type writes it — the
/// built-in default's where there is no file — put in a file beside
/// it under the document's name with `.edit` after, `$VISUAL`, else
/// `$EDITOR`, run on that file with this program's own terminal and
/// waited for, and what it left read back, parsed, and
/// [`validate`](Which::validate)d: the YAML to write. The variable is
/// split on whitespace, its first word the program and the rest its
/// arguments, so `code --wait` is an editor. Neither variable set is
/// [`Error::NoEditor`]; an editor that ends other than by success is
/// [`Error::Editor`]; a document it left that does not parse or that
/// the type refuses is that error — and in every case but success
/// nothing is written. The `.edit` file is removed on every path.
pub async fn edit(root: &Path, which: Which) -> Result<String, Error> {
    let editor = std::env::var_os("VISUAL")
        .or_else(|| std::env::var_os("EDITOR"))
        .ok_or(Error::NoEditor)?;
    let editor = editor.to_string_lossy().into_owned();
    let mut words = editor.split_whitespace();
    let Some(program) = words.next() else {
        return Err(Error::NoEditor);
    };
    let current = which.validate(read(root, which).await?)?;
    let temp = root.join(format!("{}.edit", which.file()));
    tokio::fs::write(&temp, &current).await.map_err(|source| Error::Io {
        path: temp.clone(),
        source,
    })?;
    let outcome = run(which, program, words, &temp).await;
    let _ = tokio::fs::remove_file(&temp).await;
    outcome
}

/// The editor run on the file and what it left, validated.
async fn run(which: Which, program: &str, arguments: impl Iterator<Item = &str>, temp: &Path) -> Result<String, Error> {
    let status = tokio::process::Command::new(program)
        .args(arguments)
        .arg(temp)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(Error::Spawn)?;
    if !status.success() {
        return Err(Error::Editor(status));
    }
    let bytes = tokio::fs::read(temp).await.map_err(|source| Error::Io {
        path: temp.to_path_buf(),
        source,
    })?;
    let value: serde_json::Value = serde_path_to_error::deserialize(serde_yaml_ng::Deserializer::from_slice(&bytes)).map_err(|source| Error::Parse {
        path: temp.to_path_buf(),
        source,
    })?;
    which.validate(value)
}

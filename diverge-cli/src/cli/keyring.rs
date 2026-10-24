//! The `keyring` command, done.

use std::path::Path;

use diverge_sdk::keyring::{self, Key};
use serde_json::json;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use super::Error;
use crate::arguments::Keyring;

/// Do `command` to the keyring at the root, printing one JSON line
/// per answer: `list` an entry's name and when it was made, `mint` and
/// `set` the entry kept, `remove` whether there was one. No line
/// holds a key.
pub async fn run(root: &Path, command: Keyring) -> Result<(), Error> {
    let keyring = keyring::Keyring::open(root).await?;
    let lines = match command {
        Keyring::List => keyring
            .list()
            .await?
            .into_iter()
            .map(|entry| json!({ "name": entry.name, "made": entry.made }))
            .collect(),
        Keyring::Mint { name } => {
            keyring.mint(&name).await?;
            vec![json!({ "minted": name })]
        }
        Keyring::Set { name } => {
            keyring.set(&name, read_key().await?).await?;
            vec![json!({ "set": name })]
        }
        Keyring::Remove { name } => {
            let removed = keyring.remove(&name).await?;
            vec![json!({ "removed": removed })]
        }
    };
    print(&lines).await
}

/// The key on stdin — or a password, for the issuer's identities —
/// all of it, the trailing line end dropped. An empty one is refused.
pub(super) async fn read_key() -> Result<Key, Error> {
    let mut text = String::new();
    tokio::io::stdin()
        .read_to_string(&mut text)
        .await
        .map_err(Error::Stdin)?;
    let trimmed = text.trim_end_matches(['\r', '\n']).len();
    text.truncate(trimmed);
    if text.is_empty() {
        return Err(Error::EmptyKey);
    }
    Ok(Key::given(text))
}

/// Each value as one line of compact JSON on stdout.
pub(super) async fn print(lines: &[serde_json::Value]) -> Result<(), Error> {
    let mut stdout = tokio::io::stdout();
    for line in lines {
        let mut text = line.to_string().into_bytes();
        text.push(b'\n');
        stdout.write_all(&text).await.map_err(Error::Stdout)?;
    }
    stdout.flush().await.map_err(Error::Stdout)
}

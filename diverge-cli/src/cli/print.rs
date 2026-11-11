//! What a command prints.

use tokio::io::AsyncWriteExt as _;

use super::Error;

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

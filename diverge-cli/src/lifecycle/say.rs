//! The one line a start or a stop prints.

use tokio::io::AsyncWriteExt as _;

use super::Error;

/// `line` on stdout, ended and flushed: what a start or a stop that
/// succeeded says, and the only thing either prints.
pub async fn say(line: &str) -> Result<(), Error> {
    let mut stdout = tokio::io::stdout();
    let mut text = line.as_bytes().to_vec();
    text.push(b'\n');
    stdout.write_all(&text).await.map_err(Error::Stdout)?;
    stdout.flush().await.map_err(Error::Stdout)
}

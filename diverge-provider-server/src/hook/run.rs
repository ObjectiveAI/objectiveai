//! Running the command once, and reading what it answered.

use std::path::Path;
use std::process::Stdio;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

use super::{Error, Hook};

/// The environment variable that carries the identity a question
/// concerns.
pub const IDENTITY: &str = "DIVERGE_PROVIDER_IDENTITY";

impl Hook {
    /// Run the program once with `question` as JSON on stdin and
    /// `identity` as `DIVERGE_PROVIDER_IDENTITY`, wait for it to exit,
    /// and read its stdout as `T`.
    ///
    /// Exit `0` is an answer: stdout is one JSON document, read into
    /// `T` with the path of any mismatch kept; an empty stdout is read
    /// as `null`, so a hook with nothing to say answers `()` or
    /// `None`. Every other exit, and a program that could not be
    /// started, is an [`Error`]: a hook that says no says so by
    /// exiting non-zero, and a caller reads [`Error::Status`] as no.
    ///
    /// The child is killed if this future is dropped before it exits,
    /// so an abandoned question leaks no process. Nothing times it out.
    pub async fn run<Q, T>(
        &self,
        hooks_dir: &Path,
        identity: &str,
        question: &Q,
    ) -> Result<T, Error>
    where
        Q: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let program = self.resolve(hooks_dir).ok_or(Error::Empty)?;
        let body = serde_json::to_vec(question).map_err(Error::Encode)?;

        let mut child = Command::new(&program)
            .args(self.args())
            .current_dir(hooks_dir)
            .env(IDENTITY, identity)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| Error::Spawn {
                program: program.display().to_string(),
                source,
            })?;

        // The handle is taken out and dropped after the write, so the
        // child sees EOF and knows the document is whole.
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&body).await.map_err(Error::Stdin)?;
            stdin.shutdown().await.map_err(Error::Stdin)?;
        }

        let output = child.wait_with_output().await.map_err(Error::Wait)?;
        if !output.status.success() {
            return Err(Error::Status {
                status: output.status,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        // Nothing said is `null`: `()` and `Option<T>` both read it.
        let stdout = output.stdout.trim_ascii();
        let stdout: &[u8] = if stdout.is_empty() { b"null" } else { stdout };
        let mut deserializer = serde_json::Deserializer::from_slice(stdout);
        serde_path_to_error::deserialize(&mut deserializer).map_err(Error::Parse)
    }
}

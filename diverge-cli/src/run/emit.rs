//! Printing the answers, each the moment it is had.

use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt as _;

use super::Error;
use crate::jq;

/// Where the answers go: stdout, one compact JSON line per value,
/// through the jq program when there is one. Nothing is held: a frame
/// is printed before the next is read, and a program's values are
/// written one by one as the program yields them, each line flushed
/// as it is written.
pub struct Emitter {
    /// The program, if any, compiled once for every frame.
    program: Option<jq::Program>,
    /// Stdout.
    stdout: tokio::io::Stdout,
}

impl Emitter {
    /// An emitter printing through `filter`, compiled now — so a
    /// program that does not parse or compile is refused before any
    /// request is put — or printing every frame as it is.
    pub fn new(filter: Option<String>) -> Result<Emitter, Error> {
        let program = filter.as_deref().map(jq::Program::new).transpose()?;
        Ok(Emitter {
            program,
            stdout: tokio::io::stdout(),
        })
    }

    /// The frame as JSON, printed: see [`emit_value`](Self::emit_value).
    /// The frame's JSON, for the caller to read as well.
    pub async fn emit<T: Serialize>(&mut self, frame: &T) -> Result<Value, Error> {
        let value = serde_json::to_value(frame).map_err(Error::Json)?;
        self.emit_value(value.clone()).await?;
        Ok(value)
    }

    /// The value printed: as it is, one line, when there is no
    /// program; through the program otherwise — every value it
    /// yields, one line each, written and flushed as it is yielded, a
    /// `null` left out, so a frame the program yields nothing for, or
    /// only `null`, is not printed.
    pub async fn emit_value(&mut self, value: Value) -> Result<(), Error> {
        match &self.program {
            None => line(&mut self.stdout, &value).await,
            Some(program) => {
                for value in program.run(value) {
                    let value = value?;
                    if value.is_null() {
                        continue;
                    }
                    line(&mut self.stdout, &value).await?;
                }
                Ok(())
            }
        }
    }
}

/// `value` as one compact JSON line, written and flushed.
async fn line(stdout: &mut tokio::io::Stdout, value: &Value) -> Result<(), Error> {
    let mut line = serde_json::to_vec(value).map_err(Error::Json)?;
    line.push(b'\n');
    stdout.write_all(&line).await.map_err(Error::Stdout)?;
    stdout.flush().await.map_err(Error::Stdout)
}

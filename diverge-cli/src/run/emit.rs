//! Printing the answers.

use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt as _;

use super::Error;
use crate::jq;

/// Where the answers go: stdout, one compact JSON line per value,
/// through the jq filter when there is one.
pub struct Emitter {
    /// The filter, if any.
    filter: Option<String>,
    /// Stdout.
    stdout: tokio::io::Stdout,
}

impl Emitter {
    /// An emitter printing through `filter`, or printing every frame
    /// as it is.
    pub fn new(filter: Option<String>) -> Emitter {
        Emitter {
            filter,
            stdout: tokio::io::stdout(),
        }
    }

    /// The frame as JSON, printed: see [`emit_value`](Self::emit_value).
    /// The frame's JSON, for the caller to read as well.
    pub async fn emit<T: Serialize>(&mut self, frame: &T) -> Result<Value, Error> {
        let value = serde_json::to_value(frame).map_err(Error::Json)?;
        self.emit_value(value.clone()).await?;
        Ok(value)
    }

    /// The value printed: as it is, one line, when there is no filter;
    /// through the filter otherwise — every value the filter yields,
    /// one line each, a `null` left out, so a frame the filter yields
    /// nothing for, or only `null`, is not printed.
    pub async fn emit_value(&mut self, value: Value) -> Result<(), Error> {
        let values = match &self.filter {
            None => vec![value],
            Some(filter) => jq::run(filter, value, None)?,
        };
        for value in values {
            if value.is_null() {
                continue;
            }
            let mut line = serde_json::to_vec(&value).map_err(Error::Json)?;
            line.push(b'\n');
            self.stdout.write_all(&line).await.map_err(Error::Stdout)?;
        }
        self.stdout.flush().await.map_err(Error::Stdout)
    }
}

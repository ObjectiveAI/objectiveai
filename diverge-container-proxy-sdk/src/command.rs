//! Commands: a diverge command run by the caller, its items
//! streamed back.
//!
//! A container has no CLI binary and no daemon it may dial, so a
//! command it wants run is run by the caller. The command is bytes
//! in the CLI's own vocabulary and each item is bytes in the same —
//! neither the proxy nor the caller's relay reads them — and an
//! [`Error`](Error::Command) last is the caller saying the command
//! did not finish. Nothing retries a command: it has effects, and a
//! stream that ended without its end is reported as exactly that.

use bytes::Bytes;
use diverge_provider_sdk::container_proxy::command;
use futures_util::{Stream, StreamExt as _};

use crate::{Client, Error};

impl Client {
    /// Run a diverge command, as the caller runs it: the items as they
    /// land, then the end — or, last, the caller's error.
    ///
    /// The `Err` on the way in is the ask itself failing: the call to
    /// the proxy, or a status — `502` for a command the caller could
    /// not serve or whose answer died before its first item (a
    /// command that produced nothing looks the same, by the wire's
    /// own rule). The `Err`s inside the stream are its end: the
    /// caller's [`Error::Command`], the body ending without its end
    /// record ([`Error::CommandTruncated`]), a transport failure, or
    /// a record this crate could not read. Nothing follows an `Err`.
    pub async fn command_run(
        &self,
        command: &[u8],
    ) -> Result<impl Stream<Item = Result<Bytes, Error>> + Send + use<>, Error>
    {
        let response = self
            .http()
            .post(crate::url("/command"))
            .body(command.to_vec())
            .send()
            .await
            .map_err(Error::CommandRequest)?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::CommandStatus(status.as_u16()));
        }
        let mut body = response.bytes_stream();
        Ok(async_stream::try_stream! {
            let mut buffer: Vec<u8> = Vec::new();
            loop {
                // Every whole record in the buffer, in order.
                while let Some((message, rest)) = split_record(&buffer) {
                    if message.is_empty() {
                        // The end record: the answer closed cleanly.
                        return;
                    }
                    let frame = command::response::Frame::decode(message)
                        .map_err(Error::CommandAnswer)?;
                    match frame {
                        command::response::Frame::Item(item) => {
                            let item = Bytes::copy_from_slice(item);
                            buffer = rest.to_vec();
                            yield item;
                        }
                        command::response::Frame::Error(error) => {
                            Err(Error::Command(error))?;
                            return;
                        }
                    }
                }
                match body.next().await {
                    Some(Ok(chunk)) => buffer.extend_from_slice(&chunk),
                    Some(Err(error)) => Err(Error::CommandStream(error))?,
                    None => Err(Error::CommandTruncated)?,
                }
            }
        })
    }
}

/// The bytes a record's length prefix occupies.
const LEN: usize = 4;

/// The first whole record in `buffer`, and what follows it.
fn split_record(buffer: &[u8]) -> Option<(&[u8], &[u8])> {
    let len: &[u8; LEN] = buffer.get(..LEN)?.try_into().ok()?;
    let len = u32::from_be_bytes(*len) as usize;
    let end = LEN.checked_add(len)?;
    let message = buffer.get(LEN..end)?;
    Some((message, &buffer[end..]))
}

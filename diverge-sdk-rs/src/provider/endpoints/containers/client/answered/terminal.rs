//! A provider-held terminal's answers: its id, its output, and done.

use bytes::Bytes;

use super::Answered;
use crate::shared::containers::terminal::{output, session};
use crate::shared::error::Error;
use crate::wire::decode::Decode as _;

/// `terminal_open`'s answer: the id.
#[derive(Debug, Clone, Copy)]
pub struct TerminalOpened;

impl Answered for TerminalOpened {
    type Item = u32;
    type Error = session::AnswerError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<u32, Error>, Self::Error> {
        Ok(match session::Opened::decode(&payload)? {
            session::Opened::Opened(id) => Ok(id),
            session::Opened::Error(error) => Err(error),
        })
    }
}

/// `terminal_stdin`'s, `terminal_resize`'s and `terminal_close`'s
/// answer: done.
#[derive(Debug, Clone, Copy)]
pub struct TerminalDone;

impl Answered for TerminalDone {
    type Item = ();
    type Error = session::AnswerError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<(), Error>, Self::Error> {
        Ok(match session::Answer::decode(&payload)? {
            session::Answer::Done => Ok(()),
            session::Answer::Error(error) => Err(error),
        })
    }
}

/// `terminal_attach`'s stream: every frame of the terminal's output an
/// item, its `Error` among them.
#[derive(Debug, Clone, Copy)]
pub struct TerminalOutput;

impl Answered for TerminalOutput {
    type Item = output::Owned;
    type Error = output::FrameError;
    type Refusal = Error;

    fn decode(payload: Bytes) -> Result<Result<output::Owned, Error>, Self::Error> {
        Ok(Ok(output::Frame::decode(&payload)?.owned(&payload)))
    }
}

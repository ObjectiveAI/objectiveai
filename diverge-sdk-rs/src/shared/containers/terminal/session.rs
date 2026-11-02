//! A terminal held by the provider: opened, attached, written to,
//! resized and closed by its id, on the run scope's channels.
//!
//! The rules:
//!
//! 1. `open` starts a terminal and answers its id, which names it on
//!    every channel after; it lives until `close`, until its shell
//!    exits, or until the run ends — nothing else ends it.
//! 2. `attach` streams its output: what it said while nobody was
//!    attached (its scrollback, the last [`SCROLLBACK`] bytes) first,
//!    then what it says, then how its shell ended. Dropping the attach
//!    only detaches; a second attach takes the place of the first.
//! 3. `stdin` writes bytes to it and `resize` sets its size, each
//!    answered by one [`Answer`] once done. A caller sends its next
//!    `stdin` once the last is answered, which is what keeps them in
//!    order.
//! 4. An id that names no open terminal is answered with an error.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// How many bytes of output a terminal keeps while nobody is attached:
/// the oldest go first.
pub const SCROLLBACK: usize = 1024 * 1024;

/// A terminal, by its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id {
    /// The id `open` answered.
    pub id: u32,
}

/// A terminal's new size, by its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resize {
    /// The id `open` answered.
    pub id: u32,
    /// Characters across a line.
    pub cols: u16,
    /// Lines down the screen.
    pub rows: u16,
}

/// Bytes for a terminal's stdin: its id as a `u32` big-endian, then
/// the bytes as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stdin<'a> {
    /// The id `open` answered.
    pub id: u32,
    /// What is written, verbatim.
    pub bytes: &'a [u8],
}

impl Encode for Stdin<'_> {
    /// Nothing can go wrong.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(self.bytes);
        Ok(())
    }
}

impl<'a> Decode<'a> for Stdin<'a> {
    /// Fewer than four bytes: no id.
    type Error = Short;

    fn decode(bytes: &'a [u8]) -> Result<Self, Short> {
        let (id, rest) = bytes.split_first_chunk::<4>().ok_or(Short)?;
        Ok(Stdin { id: u32::from_be_bytes(*id), bytes: rest })
    }
}

/// A stdin with no id in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Short;

impl fmt::Display for Short {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("terminal stdin holds no id")
    }
}

impl std::error::Error for Short {}

const OPENED: u8 = 0;
const DONE: u8 = 0;
const ERROR: u8 = 1;

/// What `open` is answered with: tag `0` and the id as a `u32`
/// big-endian, or tag `1` and the error's JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Opened {
    /// The terminal runs, by this id.
    Opened(u32),
    /// No terminal started, and why.
    Error(Error),
}

/// What `stdin`, `resize` and `close` are answered with: tag `0`, done,
/// or tag `1` and the error's JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// Done.
    Done,
    /// Not done, and why — an id that names no open terminal among the
    /// reasons.
    Error(Error),
}

impl Encode for Opened {
    /// The JSON failure of an error.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: a variant is called `Error`.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Opened::Opened(id) => {
                out.extend_from_slice(&[OPENED]);
                out.extend_from_slice(&id.to_be_bytes());
                Ok(())
            }
            Opened::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                serde_json::to_writer(out, error)
            }
        }
    }
}

impl Decode<'_> for Opened {
    /// See [`AnswerError`].
    type Error = AnswerError;

    fn decode(bytes: &[u8]) -> Result<Self, AnswerError> {
        let (&tag, rest) = bytes.split_first().ok_or(AnswerError::Empty)?;
        match tag {
            OPENED => rest.first_chunk::<4>().map(|id| Opened::Opened(u32::from_be_bytes(*id))).ok_or(AnswerError::Empty),
            ERROR => serde_json::from_slice(rest).map(Opened::Error).map_err(AnswerError::Body),
            other => Err(AnswerError::UnknownTag(other)),
        }
    }
}

impl Encode for Answer {
    /// The JSON failure of an error.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: a variant is called `Error`.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Answer::Done => {
                out.extend_from_slice(&[DONE]);
                Ok(())
            }
            Answer::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                serde_json::to_writer(out, error)
            }
        }
    }
}

impl Decode<'_> for Answer {
    /// See [`AnswerError`].
    type Error = AnswerError;

    fn decode(bytes: &[u8]) -> Result<Self, AnswerError> {
        let (&tag, rest) = bytes.split_first().ok_or(AnswerError::Empty)?;
        match tag {
            DONE => Ok(Answer::Done),
            ERROR => serde_json::from_slice(rest).map(Answer::Error).map_err(AnswerError::Body),
            other => Err(AnswerError::UnknownTag(other)),
        }
    }
}

/// An answer that did not decode.
#[derive(Debug)]
pub enum AnswerError {
    /// No tag, or no id after it.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
    /// An error whose JSON did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for AnswerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnswerError::Empty => f.write_str("terminal answer is empty"),
            AnswerError::UnknownTag(tag) => write!(f, "unknown terminal answer tag {tag}"),
            AnswerError::Body(error) => write!(f, "terminal answer did not parse: {error}"),
        }
    }
}

impl std::error::Error for AnswerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AnswerError::Body(error) => Some(error),
            AnswerError::Empty | AnswerError::UnknownTag(_) => None,
        }
    }
}

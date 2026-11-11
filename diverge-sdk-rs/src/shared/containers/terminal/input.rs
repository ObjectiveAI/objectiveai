//! What goes into a terminal.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

const STDIN: u8 = 0;
const RESIZE: u8 = 1;
const CANCEL: u8 = 2;

/// One thing a caller puts into a terminal: one byte of tag, then the
/// bytes as they are for `Stdin`, JSON for `Resize`, nothing for `Cancel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input<'a> {
    /// Tag `0`: bytes written to the terminal, in the order sent.
    Stdin(&'a [u8]),
    /// Tag `1`: the terminal's new size.
    Resize(super::open::Open),
    /// Tag `2`: kill the shell.
    Cancel,
}

impl Encode for Input<'_> {
    /// The JSON failure of a size.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Input::Stdin(bytes) => {
                out.extend_from_slice(&[STDIN]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Input::Resize(size) => {
                out.extend_from_slice(&[RESIZE]);
                serde_json::to_writer(out, size)
            }
            Input::Cancel => {
                out.extend_from_slice(&[CANCEL]);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Input<'a> {
    /// See [`InputError`].
    type Error = InputError;

    fn decode(bytes: &'a [u8]) -> Result<Self, InputError> {
        let (&tag, rest) = bytes.split_first().ok_or(InputError::Empty)?;
        Ok(match tag {
            STDIN => Input::Stdin(rest),
            RESIZE => Input::Resize(serde_json::from_slice(rest).map_err(InputError::Body)?),
            CANCEL => Input::Cancel,
            other => return Err(InputError::UnknownTag(other)),
        })
    }
}

/// Input that did not decode.
#[derive(Debug)]
pub enum InputError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
    /// A size whose JSON did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::Empty => f.write_str("terminal input is empty"),
            InputError::UnknownTag(tag) => write!(f, "unknown terminal input tag {tag}"),
            InputError::Body(error) => write!(f, "terminal size did not parse: {error}"),
        }
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InputError::Body(error) => Some(error),
            InputError::Empty | InputError::UnknownTag(_) => None,
        }
    }
}

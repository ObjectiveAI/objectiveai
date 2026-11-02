//! What goes into a frontend request after its head.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

const BODY: u8 = 0;
const END: u8 = 1;
const STOP: u8 = 2;

/// One thing a caller puts into a frontend request: one byte of tag,
/// then the bytes as they are for `Body`, nothing for the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input<'a> {
    /// Tag `0`: a piece of the request's body, or, after a `101`, of
    /// what the page sends.
    Body(&'a [u8]),
    /// Tag `1`: the body is done.
    End,
    /// Tag `2`: drop the request.
    Stop,
}

impl Encode for Input<'_> {
    /// Nothing can go wrong.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Input::Body(bytes) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(bytes);
            }
            Input::End => out.extend_from_slice(&[END]),
            Input::Stop => out.extend_from_slice(&[STOP]),
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Input<'a> {
    /// See [`InputError`].
    type Error = InputError;

    fn decode(bytes: &'a [u8]) -> Result<Self, InputError> {
        let (&tag, rest) = bytes.split_first().ok_or(InputError::Empty)?;
        Ok(match tag {
            BODY => Input::Body(rest),
            END => Input::End,
            STOP => Input::Stop,
            other => return Err(InputError::UnknownTag(other)),
        })
    }
}

/// Input that did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::Empty => f.write_str("frontend input is empty"),
            InputError::UnknownTag(tag) => write!(f, "unknown frontend input tag {tag}"),
        }
    }
}

impl std::error::Error for InputError {}

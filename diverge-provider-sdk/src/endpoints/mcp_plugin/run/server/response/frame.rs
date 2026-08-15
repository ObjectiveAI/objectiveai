//! What a server's response frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The plugin is running.
///
/// One of these, on channel `0`, once the container is up and its MCP
/// server can be reached. It carries nothing, because saying so IS the
/// whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | a [`Frame`], then work | the plugin is up; call it |
/// | a finish, and nothing before it | it never came up |
/// | nothing | the connection died; whether it came up is unknowable from here |
///
/// # Why there is no id
///
/// A [`laboratory run`](crate::endpoints::laboratories::run)
/// answers with one, because a laboratory is a place others join: an
/// id is what a
/// [`connect`](crate::endpoints::laboratories::connect) names and what
/// a [`transfer`](crate::shared::container::transfer) sends a file to.
///
/// A plugin is none of those. It is created for one caller, answers
/// that caller's tool calls, and is torn down after — so the scope is
/// the whole of the handle, and an id would name a thing nobody can
/// address. Minting one anyway would be inventing a way to reach a
/// plugin container that this specification deliberately does not
/// offer.
///
/// # Why there is no filetree either
///
/// Because nothing reads it. A laboratory reports its filesystem
/// because an agent works in it and wants to see what it is working
/// on. A plugin serves tools; its filesystem is its author's business,
/// it takes no [`mounts`], and a caller with no way to read or write
/// inside it has nothing to do with a tree of it.
///
/// # It still spends a byte
///
/// A payload of zero bytes would carry the same information today and
/// cost a wire break tomorrow. Failure has no shape yet — a provider
/// says so by finishing the scope without this — and when it gets one
/// it will be another tag value, which is only additive if there is a
/// tag to add to.
///
/// The same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one.
///
/// [`mounts`]: crate::endpoints::laboratories::run::client::request::Frame::mounts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says the plugin is running.
const READY: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[READY]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&READY) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// An MCP plugin response frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a provider reporting a failure will send, once
    /// failures have a shape. Until then it is a peer that disagrees
    /// about the protocol.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin response frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}

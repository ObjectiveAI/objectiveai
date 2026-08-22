//! One message on the conduit, and what it takes to read a stream of
//! them.

use std::error;
use std::fmt;

use bytes::{Buf as _, Bytes, BytesMut};
use futures_util::{Stream, StreamExt as _, stream};

/// The bytes in front of every message: a length and an exchange.
pub const HEADER_LEN: usize = 4 + 4;

/// One message, in whichever direction it was travelling.
///
/// The same shape both ways, because the framing has nothing to do with
/// what is being framed — see [`mcp_conduit`](super) for what a payload
/// holds going up and what it holds coming down.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Message {
    /// Which exchange this belongs to.
    ///
    /// Chosen by the container, which is the end that starts one. Two
    /// live exchanges sharing a number is the container's mistake and
    /// nothing here can catch it: the provider would answer both into
    /// the same conversation, and only the party that minted the
    /// number could have prevented it. The same terms a
    /// [`write_id`](crate::shared::container::write_path::request::Request::write_id)
    /// is on.
    ///
    /// Free for reuse once the exchange has finished. Nothing here
    /// remembers.
    pub exchange: u32,
    /// The message's own bytes.
    ///
    /// Empty means the exchange is over, which only travels downward —
    /// see [`mcp_conduit`](super).
    pub payload: Bytes,
}

/// Write one message.
///
/// The length counts the payload alone. It is the one number a reader
/// needs and the one a writer already has, and counting the header into
/// it would mean subtracting a constant at every use.
pub fn encode(exchange: u32, payload: &[u8]) -> Bytes {
    let mut bytes = BytesMut::with_capacity(HEADER_LEN + payload.len());
    bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&exchange.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes.freeze()
}

/// Read a pipe as the messages on it.
///
/// A byte pipe has no message boundaries, so what arrives is chunks
/// that split and join wherever the network felt like it. This holds
/// the leftovers and yields a [`Message`] each time a whole one is
/// there.
///
/// # It ends where the pipe ends
///
/// Cleanly if the last message was complete, and with
/// [`Truncated`](MessageError::Truncated) if there were bytes left
/// over — a container that stopped mid-message, which is worth telling
/// apart from one that stopped between them.
///
/// An error is final either way. Nothing is yielded after one, because
/// a pipe that failed once has no position left to resume from and a
/// reader that kept going would be reading whatever the next bytes
/// happened to be.
pub fn messages<S, E>(
    pipe: S,
) -> impl Stream<Item = Result<Message, MessageError<E>>>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    stream::unfold(
        (pipe, BytesMut::new(), false),
        |(mut pipe, mut buffer, done)| async move {
            if done {
                return None;
            }
            loop {
                if buffer.len() >= HEADER_LEN {
                    let length = u32::from_be_bytes(
                        buffer[..4].try_into().expect("four bytes"),
                    ) as usize;
                    if buffer.len() >= HEADER_LEN + length {
                        let exchange = u32::from_be_bytes(
                            buffer[4..HEADER_LEN]
                                .try_into()
                                .expect("four bytes"),
                        );
                        buffer.advance(HEADER_LEN);
                        let payload = buffer.split_to(length).freeze();
                        let message = Message { exchange, payload };
                        return Some((Ok(message), (pipe, buffer, false)));
                    }
                }
                match pipe.next().await {
                    Some(Ok(chunk)) => buffer.extend_from_slice(&chunk),
                    Some(Err(error)) => {
                        let error = MessageError::Pipe(error);
                        return Some((Err(error), (pipe, buffer, true)));
                    }
                    None if buffer.is_empty() => return None,
                    None => {
                        let error = MessageError::Truncated(buffer.len());
                        return Some((Err(error), (pipe, buffer, true)));
                    }
                }
            }
        },
    )
}

/// A conduit that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageError<E> {
    /// The pipe itself failed, carrying whatever the container's
    /// runtime said about it.
    Pipe(E),
    /// The pipe ended part way through a message, carrying how many
    /// bytes were left with nowhere to go.
    ///
    /// Distinct from ending cleanly, and the distinction is the point:
    /// a container that exits between messages has finished, and one
    /// that exits inside a message has crashed.
    Truncated(usize),
}

impl<E: fmt::Display> fmt::Display for MessageError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::Pipe(error) => {
                write!(f, "the mcp conduit failed: {error}")
            }
            MessageError::Truncated(len) => {
                write!(f, "the mcp conduit ended with {len} bytes unread")
            }
        }
    }
}

impl<E: error::Error + 'static> error::Error for MessageError<E> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            MessageError::Pipe(error) => Some(error),
            MessageError::Truncated(_) => None,
        }
    }
}

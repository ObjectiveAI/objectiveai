//! The proxy↔host wire format.
//!
//! Defined HERE, and deliberately NOT in the SDK. The format is a
//! private contract between two binaries; the laboratory host declares
//! its own copy of it. Publishing it through the SDK would make every
//! adjustment a breaking change for every SDK consumer, none of whom
//! have any business speaking it.
//!
//! Every message on the socket is a WebSocket BINARY frame:
//!
//! ```text
//! [tag: u8][id: u32 big-endian][payload…]
//! ```
//!
//! | tag | frame   | payload | direction   |
//! |-----|---------|---------|-------------|
//! | 0   | `Open`  | none    | proxy → host |
//! | 1   | `Data`  | bytes   | both        |
//! | 2   | `Close` | none    | both        |
//!
//! There are no text frames: one uniform shape means neither end needs
//! a second parser.
//!
//! Payloads are OPAQUE. pgwire is never parsed here, so TLS
//! negotiation and every protocol extension cross untouched, and a
//! Postgres message larger than one frame simply spans several — both
//! ends reassemble from a byte stream, exactly as they would from a
//! socket.

use axum::body::Bytes;

pub const TAG_OPEN: u8 = 0;
pub const TAG_DATA: u8 = 1;
pub const TAG_CLOSE: u8 = 2;

/// `tag` + `id`. Five bytes, which is already noise beside a WebSocket
/// frame's own 2–14 byte header — the reason `id` is a comfortable
/// `u32` instead of a `u16` with wraparound bookkeeping.
pub const HEADER_LEN: usize = 5;

#[derive(Debug)]
pub enum Frame {
    Open { id: u32 },
    Data { id: u32, payload: Bytes },
    Close { id: u32 },
}

impl Frame {
    /// Decode one binary message. `None` for anything malformed or
    /// tagged unknown — a frame this build cannot read is not one it
    /// can route, and the socket carries other streams that can.
    ///
    /// `Bytes` in, `Bytes` out: the payload is a view into the same
    /// allocation the socket delivered, so demuxing costs no copy.
    pub fn decode(bytes: Bytes) -> Option<Self> {
        if bytes.len() < HEADER_LEN {
            return None;
        }
        let id = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        match bytes[0] {
            TAG_OPEN => Some(Frame::Open { id }),
            TAG_DATA => Some(Frame::Data {
                id,
                payload: bytes.slice(HEADER_LEN..),
            }),
            TAG_CLOSE => Some(Frame::Close { id }),
            _ => None,
        }
    }
}

/// Frame a freshly-read chunk. Takes a slice rather than `Frame::Data`
/// because this is the hot path — going through the enum would buy an
/// extra copy of every byte the database ever sends.
pub fn encode_data(id: u32, payload: &[u8]) -> Bytes {
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.push(TAG_DATA);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(payload);
    Bytes::from(out)
}

pub fn encode_open(id: u32) -> Bytes {
    encode_control(TAG_OPEN, id)
}

pub fn encode_close(id: u32) -> Bytes {
    encode_control(TAG_CLOSE, id)
}

fn encode_control(tag: u8, id: u32) -> Bytes {
    let mut out = [0u8; HEADER_LEN];
    out[0] = tag;
    out[1..].copy_from_slice(&id.to_be_bytes());
    Bytes::copy_from_slice(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_frames_round_trip() {
        assert!(matches!(
            Frame::decode(encode_open(1)),
            Some(Frame::Open { id: 1 })
        ));
        assert!(matches!(
            Frame::decode(encode_close(u32::MAX)),
            Some(Frame::Close { id: u32::MAX })
        ));
    }

    #[test]
    fn data_frames_round_trip() {
        let bytes = encode_data(0x0102_0304, b"pgwire");
        assert_eq!(bytes.len(), HEADER_LEN + 6);
        match Frame::decode(bytes) {
            Some(Frame::Data { id, payload }) => {
                assert_eq!(id, 0x0102_0304);
                assert_eq!(&payload[..], b"pgwire");
            }
            other => panic!("expected Data, got {other:?}"),
        }
    }

    /// An empty `Data` payload is legal and distinct from `Close`: a
    /// zero-length write is not an end of stream.
    #[test]
    fn empty_data_is_not_close() {
        match Frame::decode(encode_data(7, b"")) {
            Some(Frame::Data { id: 7, payload }) => assert!(payload.is_empty()),
            other => panic!("expected empty Data, got {other:?}"),
        }
    }

    #[test]
    fn short_and_unknown_frames_are_rejected() {
        assert!(Frame::decode(Bytes::from_static(&[TAG_DATA, 0, 0, 0])).is_none());
        assert!(Frame::decode(Bytes::new()).is_none());
        assert!(Frame::decode(Bytes::from_static(&[200, 0, 0, 0, 1])).is_none());
    }
}

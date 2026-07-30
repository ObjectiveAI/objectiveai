//! The host's half of the `objectiveai-db-proxy` wire format.
//!
//! Declared HERE, independently of the crate that defines the other
//! half, and deliberately NOT shared through the SDK: the format is a
//! private contract between two binaries, and publishing it would make
//! every adjustment a breaking change for every SDK consumer, none of
//! whom speak it.
//!
//! Every message on the socket is a WebSocket BINARY frame:
//!
//! ```text
//! [tag: u8][id: u32 big-endian][payload…]
//! ```
//!
//! | tag | frame   | payload | direction    |
//! |-----|---------|---------|--------------|
//! | 0   | `Open`  | none    | proxy → host |
//! | 1   | `Data`  | bytes   | both         |
//! | 2   | `Close` | none    | both         |
//!
//! The `id` names one Postgres connection inside the container, minted
//! by the proxy because the proxy owns the accept. This side never
//! creates one, which is why there is no `encode_open` here — only the
//! decode, and the two frames the host sends.
//!
//! Payloads are OPAQUE: pgwire is never parsed on either side, so TLS
//! negotiation and every protocol extension cross untouched, and a
//! Postgres message larger than one frame simply spans several.

const TAG_OPEN: u8 = 0;
const TAG_DATA: u8 = 1;
const TAG_CLOSE: u8 = 2;

/// `tag` + `id`.
const HEADER_LEN: usize = 5;

#[derive(Debug)]
pub enum Frame {
    Open { id: u32 },
    Data { id: u32, bytes: Vec<u8> },
    Close { id: u32 },
}

impl Frame {
    /// Decode one binary message. `None` for anything malformed or
    /// tagged unknown — a frame this build cannot read is not one it can
    /// route, and the socket carries other streams that it can.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_LEN {
            return None;
        }
        let id = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        match bytes[0] {
            TAG_OPEN => Some(Frame::Open { id }),
            TAG_DATA => Some(Frame::Data {
                id,
                bytes: bytes[HEADER_LEN..].to_vec(),
            }),
            TAG_CLOSE => Some(Frame::Close { id }),
            _ => None,
        }
    }
}

/// Frame database bytes bound for one container connection.
pub fn encode_data(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.push(TAG_DATA);
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Tell the proxy this stream is over, so it shuts the client socket.
pub fn encode_close(id: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.push(TAG_CLOSE);
    out.extend_from_slice(&id.to_be_bytes());
    out
}

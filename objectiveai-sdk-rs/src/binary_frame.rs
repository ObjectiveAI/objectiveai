//! The binary wire frame: `[u32 BE header length][JSON header][raw
//! payload bytes]`, all in ONE WebSocket binary message.
//!
//! Bulk payloads (laboratory-transfer chunks) never ride base64 inside
//! JSON — the typed envelope a text frame would carry serializes
//! UNCHANGED as the header (its `data` field is `#[serde(skip)]`), and
//! the raw bytes follow out of band. Correlation is exactly the text
//! frame's: the header holds the same ids. A receiver reads the
//! length, parses the header with the same serde types it already
//! uses, and attaches the remaining bytes to the chunk field.
//!
//! Std-only and transport-agnostic — both WebSocket hops (daemon ↔
//! laboratory host, API ↔ daemon reverse channel) share it.

/// One decoded-or-encoded wire frame: JSON text for every ordinary
/// message, the binary sandwich for chunk-bearing ones.
#[derive(Debug, Clone)]
pub enum WireFrame {
    Text(String),
    Binary(Vec<u8>),
}

/// Build the binary sandwich. `header` is the frame's full JSON
/// envelope (minus the payload bytes, which its serde representation
/// skips); `payload` may be empty (chunk variants are ALWAYS binary,
/// content notwithstanding — the framing is variant-keyed, never
/// content-sniffed).
pub fn encode(header: &str, payload: &[u8]) -> Vec<u8> {
    let header = header.as_bytes();
    let mut frame = Vec::with_capacity(4 + header.len() + payload.len());
    frame.extend_from_slice(&(header.len() as u32).to_be_bytes());
    frame.extend_from_slice(header);
    frame.extend_from_slice(payload);
    frame
}

/// Split a binary sandwich back into `(header JSON, payload bytes)`.
/// `None` on anything malformed — a short frame, a header length past
/// the end, or a non-UTF-8 header. Malformed frames are DROPPED by
/// receivers (forward-compat posture, mirroring unknown text frames).
pub fn decode(frame: &[u8]) -> Option<(&str, &[u8])> {
    let len_bytes: [u8; 4] = frame.get(..4)?.try_into().ok()?;
    let header_len = u32::from_be_bytes(len_bytes) as usize;
    let header_end = 4usize.checked_add(header_len)?;
    let header = std::str::from_utf8(frame.get(4..header_end)?).ok()?;
    let payload = frame.get(header_end..)?;
    Some((header, payload))
}

//! Decoder for the composite laboratory id
//! `{machineID}/{base62(state)}/{base62(laboratoryID)}` that arrives
//! in `OBJECTIVEAI_LABORATORY_ID`.
//!
//! LOCAL MIRROR of the canonical implementation in
//! `objectiveai-sdk-rs/src/laboratories/composite.rs`
//! (`ClientLaboratory::composite_id` / `from_composite_id`) — this
//! container binary deliberately does not depend on the SDK (it is a
//! minimal zig-built binary injected into laboratory containers).
//! Keep the alphabet and split rules in sync with the SDK.
//!
//! Only the DECODE direction lives here: the host encodes at
//! container-create time; this binary parses the value back to name
//! its MCP server `oail-<raw id>` (tool names cannot carry the
//! composite — the 64-hex machine id busts provider name limits) and
//! to surface the composite verbatim in the server instructions.

/// The base62 digit alphabet, in the `base62` crate's default order.
const ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Decode a base62 segment back to the original string. `None` on any
/// character outside the alphabet or a byte sequence that is not
/// valid UTF-8. Leading `'0'` digits map back to leading 0x00 bytes.
fn base62_decode_str(s: &str) -> Option<String> {
    let digits = s.as_bytes();
    let leading_zeros = digits.iter().take_while(|d| **d == b'0').count();
    let mut num: Vec<u8> = Vec::new();
    for digit in &digits[leading_zeros..] {
        let value = ALPHABET.iter().position(|a| a == digit)? as u32;
        // num := num * 62 + value, over base-256 limbs.
        let mut carry = value;
        for byte in num.iter_mut().rev() {
            let acc = *byte as u32 * 62 + carry;
            *byte = (acc % 256) as u8;
            carry = acc / 256;
        }
        while carry > 0 {
            num.insert(0, (carry % 256) as u8);
            carry /= 256;
        }
    }
    let mut bytes = vec![0u8; leading_zeros];
    bytes.extend(num);
    String::from_utf8(bytes).ok()
}

/// Parse a composite laboratory id into `(machine, state, raw id)`.
/// `None` on anything that is not EXACTLY three `/`-separated
/// segments with a non-empty machine id and base62-decodable state +
/// laboratory id — safe because the machine segment is hex and the
/// encoded segments are pure base62, so neither can contain a `/`.
pub fn parse_composite_laboratory_id(s: &str) -> Option<(String, String, String)> {
    let mut parts = s.split('/');
    let machine = parts.next()?;
    let machine_state = parts.next()?;
    let id = parts.next()?;
    if parts.next().is_some() || machine.is_empty() {
        return None;
    }
    Some((
        machine.to_string(),
        base62_decode_str(machine_state)?,
        base62_decode_str(id)?,
    ))
}

/// 32-bit FNV-1a — mirrors the SDK's server-name hash exactly.
fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// A u32 as exactly 6 base62 digits (62^6 > 2^32), zero-padded.
fn base62_encode_u32(mut value: u32) -> String {
    let mut digits = [b'0'; 6];
    for slot in digits.iter_mut().rev() {
        *slot = ALPHABET[(value % 62) as usize];
        value /= 62;
    }
    String::from_utf8(digits.to_vec()).expect("alphabet is ASCII")
}

/// The MCP SERVER NAME for this laboratory —
/// `oail-<base62(fnv1a32(env value))>`, a fixed 11-char
/// `[0-9A-Za-z-]` token (mirrors
/// `ClientLaboratory::server_name` in the SDK, which hashes the same
/// composite string). Server names feed the proxy's tool-name prefix,
/// which is bound by provider tool-name limits — a hash is always
/// safe regardless of the raw id's length or charset.
pub fn server_name(env_value: &str) -> String {
    format!("oail-{}", base62_encode_u32(fnv1a32(env_value.as_bytes())))
}

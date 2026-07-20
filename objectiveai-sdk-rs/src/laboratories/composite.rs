//! The ASSISTANT-FACING composite laboratory id:
//! `{machineID}/{base62(state)}/{base62(laboratoryID)}`.
//!
//! Laboratory ids are only unique per (machine, state), so the id an
//! assistant quotes must carry the whole identity. The machine id is
//! 64 lowercase hex chars ([`crate::machine::machine_id`]) — never a
//! `/`; state and laboratory id are ARBITRARY strings (they may
//! contain `/`), so both are base62-encoded (alphabet `0-9A-Za-z`,
//! the `base62` crate's digit order) — after splitting on `/` there
//! are always EXACTLY three unambiguous segments.
//!
//! The composite is a PRESENTATION at the assistant boundary only:
//! the raw id stays raw on every other surface (DB rows, commands,
//! HostIdentify, `client://laboratory/{id}`, the `oail-<rawid>` tool
//! prefix — tool names cannot carry a 64-hex machine id under
//! provider name limits).
//!
//! The in-container `objectiveai-mcp-laboratory` binary keeps a local
//! mirror of the DECODER (it deliberately does not depend on this
//! crate) — keep the two in sync.

use super::{ClientLaboratory, ClientLaboratoryType};

/// The base62 digit alphabet, in the `base62` crate's default order.
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Encode arbitrary bytes as base62 (big-endian big-integer base
/// conversion). Leading 0x00 bytes are preserved as leading `'0'`
/// digits; the empty string encodes to the empty string.
fn base62_encode_str(s: &str) -> String {
    let bytes = s.as_bytes();
    let leading_zeros = bytes.iter().take_while(|b| **b == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    let mut num: Vec<u8> = bytes[leading_zeros..].to_vec();
    while !num.is_empty() {
        // One long-division pass: num := num / 62, remainder → digit.
        let mut quotient: Vec<u8> = Vec::with_capacity(num.len());
        let mut remainder: u32 = 0;
        for byte in &num {
            let acc = remainder * 256 + *byte as u32;
            let q = (acc / 62) as u8;
            remainder = acc % 62;
            if !(quotient.is_empty() && q == 0) {
                quotient.push(q);
            }
        }
        digits.push(ALPHABET[remainder as usize]);
        num = quotient;
    }
    let mut out = String::with_capacity(leading_zeros + digits.len());
    for _ in 0..leading_zeros {
        out.push('0');
    }
    out.extend(digits.iter().rev().map(|d| *d as char));
    out
}

/// Decode a [`base62_encode_str`] string back to the original. `None`
/// on any character outside the alphabet or a byte sequence that is
/// not valid UTF-8.
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
    // A '0' digit inside the number contributes nothing extra; only
    // LEADING '0's map back to leading 0x00 bytes.
    let mut bytes = vec![0u8; leading_zeros];
    bytes.extend(num);
    String::from_utf8(bytes).ok()
}

/// 32-bit FNV-1a — tiny, dependency-free, and trivially mirrored in
/// the in-container binary. Only used to derive short display-safe
/// server names; NOT a security or content-addressing hash.
fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in bytes {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// A u32 as exactly 6 base62 digits (62^6 > 2^32), zero-padded — the
/// fixed-width, `[0-9A-Za-z]`-only token server names are built from.
fn base62_encode_u32(mut value: u32) -> String {
    let mut digits = [b'0'; 6];
    for slot in digits.iter_mut().rev() {
        *slot = ALPHABET[(value % 62) as usize];
        value /= 62;
    }
    String::from_utf8(digits.to_vec()).expect("alphabet is ASCII")
}

impl ClientLaboratory {
    /// The laboratory's MCP SERVER NAME — `oail-<base62(fnv1a32(composite))>`,
    /// a fixed 11-char `[0-9A-Za-z-]` token. Server names feed the
    /// proxy's tool-name prefix (`<name>_Bash`), which is bound by
    /// provider tool-name limits (length + charset) — so the name is a
    /// HASH of the [composite id](Self::composite_id), never the raw
    /// id (arbitrary length/charset) and never the composite itself
    /// (the 64-hex machine id alone busts the limits). `None` when the
    /// marker predates machine tracking (no composite to hash). The
    /// in-container binary computes the same name from its
    /// `OBJECTIVEAI_LABORATORY_ID` env — keep the mirrors in sync.
    pub fn server_name(&self) -> Option<String> {
        self.composite_id()
            .map(|composite| format!("oail-{}", base62_encode_u32(fnv1a32(composite.as_bytes()))))
    }

    /// The assistant-facing composite id
    /// `{machineID}/{base62(state)}/{base62(laboratoryID)}` — `None`
    /// when the marker predates machine tracking (no pair to compose).
    pub fn composite_id(&self) -> Option<String> {
        let machine = self.machine.as_deref()?;
        let machine_state = self.machine_state.as_deref()?;
        Some(format!(
            "{machine}/{}/{}",
            base62_encode_str(machine_state),
            base62_encode_str(&self.id)
        ))
    }

    /// Parse a [`composite_id`](Self::composite_id) back into a
    /// marker (with the pair always `Some`). `None` on anything that
    /// is not EXACTLY three `/`-separated segments with a non-empty
    /// machine id and base62-decodable state + laboratory id — safe
    /// because the machine segment is hex and the encoded segments
    /// are pure base62, so neither can contain a `/` of its own.
    pub fn from_composite_id(s: &str) -> Option<Self> {
        let mut parts = s.split('/');
        let machine = parts.next()?;
        let machine_state = parts.next()?;
        let id = parts.next()?;
        if parts.next().is_some() || machine.is_empty() {
            return None;
        }
        Some(Self {
            r#type: ClientLaboratoryType::Client,
            id: base62_decode_str(id)?,
            machine: Some(machine.to_string()),
            machine_state: Some(base62_decode_str(machine_state)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(machine: &str, state: &str, id: &str) -> ClientLaboratory {
        ClientLaboratory {
            r#type: ClientLaboratoryType::Client,
            id: id.to_string(),
            machine: Some(machine.to_string()),
            machine_state: Some(state.to_string()),
        }
    }

    #[test]
    fn roundtrips() {
        for (state, id) in [
            ("default", "my-lab"),
            ("state/with/slashes", "id/with/slashes"),
            ("under_score.dots", "0leading-zero-digit"),
            ("ünïcodé 状態", "研究室/λ"),
            ("", "empty-state"),
            ("s", ""),
            ("\u{0}\u{0}leading-nuls", "trailing-nul\u{0}"),
        ] {
            let original = marker("ab12", state, id);
            let composite = original.composite_id().expect("pair present");
            assert_eq!(
                composite.matches('/').count(),
                2,
                "exactly three segments: {composite}"
            );
            let parsed = ClientLaboratory::from_composite_id(&composite)
                .unwrap_or_else(|| panic!("parses: {composite}"));
            assert_eq!(parsed, original, "roundtrip for {state:?}/{id:?}");
        }
    }

    #[test]
    fn server_name_is_fixed_width_and_name_safe() {
        // Arbitrary-length/charset raw ids all hash to the same shape.
        for (state, id) in [
            ("default", "my-lab"),
            ("state/with/slashes", &"x".repeat(500) as &str),
            ("ünïcodé 状態", "研究室/λ"),
        ] {
            let lab = marker("3fa8", state, id);
            let name = lab.server_name().expect("pair present");
            assert_eq!(name.len(), "oail-".len() + 6, "fixed width: {name}");
            assert!(name.starts_with("oail-"));
            assert!(
                name["oail-".len()..].bytes().all(|b| b.is_ascii_alphanumeric()),
                "hash token is [0-9A-Za-z]: {name}"
            );
            // Deterministic.
            assert_eq!(lab.server_name(), Some(name));
        }
        // Pair-less markers have no composite to hash.
        let mut lab = marker("m", "s", "i");
        lab.machine = None;
        assert_eq!(lab.server_name(), None);
    }

    #[test]
    fn pairless_marker_has_no_composite() {
        let mut lab = marker("m", "s", "i");
        lab.machine_state = None;
        assert_eq!(lab.composite_id(), None);
        lab.machine_state = Some("s".to_string());
        lab.machine = None;
        assert_eq!(lab.composite_id(), None);
    }

    #[test]
    fn malformed_inputs_rejected() {
        for bad in [
            "",                    // no segments
            "bare-id",             // one segment
            "machine/only-two",    // two segments
            "m/a/b/c",             // four segments
            "/state62/id62",       // empty machine
            "m/bad!chars/aa",      // outside the alphabet
            "m/aa/bad chars",      // outside the alphabet
        ] {
            assert_eq!(
                ClientLaboratory::from_composite_id(bad),
                None,
                "must reject {bad:?}"
            );
        }
    }
}

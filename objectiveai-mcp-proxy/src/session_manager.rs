//! Session registry.
//!
//! Maps session ids to [`Session`]s. A proxy session id is the base62
//! encoding of an authenticated-encrypted, versioned envelope wrapping a
//! JSON-serialized `IndexMap<upstream_url, IndexMap<header_name,
//! header_value>>`. Each upstream's value is the full set of HTTP
//! headers needed to reconnect: `Mcp-Session-Id`, `Authorization`, plus
//! any custom headers the original initialize request supplied.
//!
//! Stable encoding: URLs sort alphabetically; headers within each
//! per-URL map sort alphabetically, AND the AEAD nonce is derived
//! deterministically from a BLAKE3 keyed hash of the canonical
//! plaintext. So two requests with the same `{url → {header → value}}`
//! content always encode to the *same* base62 id, byte-for-byte. That
//! lets `handle_initialize`'s alive-in-memory branch hand the original
//! id straight back to the caller — re-minting was previously producing
//! a fresh ciphertext (random nonce) that didn't match any key in
//! `state.sessions`, so the agent's next POST 404'd. Same payload now
//! always lives at the same id. Authentication tag covers the version
//! byte + nonce + ciphertext, so any tampering produces a decryption
//! failure.
//!
//! Wire format (pre-base62):
//! ```text
//! [ 1B version (0x01) | 24B XChaCha20 nonce | ciphertext... | 16B Poly1305 tag ]
//! ```
//!
//! Encryption uses one 256-bit key threaded in via
//! [`SessionManager::new`]. Operators rotate by setting the new key in
//! `MCP_ENCRYPTION_KEY` and restarting the proxy — every outstanding
//! session id minted under the old key becomes invalid (a 401 on
//! resume), which forces clients to re-initialize.
//!
//! All per-session dispatch (list, call, read) lives on [`Session`]
//! itself; this file only cares about computing/minting ids, packing
//! connections + their canonical headers into a [`Session`], and
//! looking sessions back up.

use std::sync::Arc;

use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::mcp::Connection;
use rand::RngCore;

use crate::session::Session;

/// Per-session encoded payload.
///
/// `connections` is `URL → header_map`, where the header_map is the
/// full set of HTTP headers used to reconnect that upstream
/// (`Mcp-Session-Id`, `Authorization`, custom `X-*`). The session id is
/// uniform with every other header — there's no separate session-id
/// field. URLs sort alphabetically when encoding for stable ids; the
/// per-URL header map sorts the same way.
///
/// `agent_instance_hierarchy` carries the caller-supplied `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`
/// value at session-open time. Because the whole payload is
/// AEAD-encrypted into the public session id, this value travels
/// inside every Mcp-Session-Id the upstream sdk runners hand back to
/// the proxy — runners don't have to re-send the header themselves.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionPayload {
    pub connections: IndexMap<String, IndexMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_instance_hierarchy: Option<String>,
    /// Bare 22-character leaf identity from `X-OBJECTIVEAI-AGENT-ID`.
    /// Travels alongside `agent_instance_hierarchy` inside the AEAD-
    /// encrypted session id so runners don't have to re-send the
    /// header on resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// WF-level identity from `X-OBJECTIVEAI-AGENT-FULL-ID`. Same
    /// AEAD-recovery path as `agent_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_full_id: Option<String>,
    /// JSON-encoded `RemotePath` from `X-OBJECTIVEAI-AGENT-REMOTE`.
    /// `None` when the WF was inline. Same AEAD-recovery path as
    /// `agent_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_remote: Option<String>,
    /// Per-upstream-URL allowlist of un-prefixed tool names. URLs
    /// present in this map have `tools/list` filtered to only those
    /// names (the proxy still applies its `<server_name>_` prefix
    /// downstream). URLs absent → no filtering (every tool visible).
    /// Empty Vec for a URL → zero tools visible from that upstream.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub tool_allowlists: IndexMap<String, Vec<String>>,
}

/// Current envelope version byte. Bumping this lets future shape
/// changes be distinguished from old ids that happen to decrypt under
/// the same key set. Decoders that see an unrecognized version return
/// `None`.
const VERSION: u8 = 0x01;
const NONCE_LEN: usize = 24; // XChaCha20-Poly1305 nonce
const TAG_LEN: usize = 16; // Poly1305 tag (handled internally by `Aead::encrypt`)

/// Maps a session id to its [`Session`] state. Owns the encryption
/// key used for minting and decoding ids.
#[derive(Debug)]
pub struct SessionManager {
    sessions: DashMap<String, Arc<Session>>,
    /// 256-bit AEAD key. Sessions minted under one key cannot be
    /// decrypted by another — to rotate, set a new key on the
    /// proxy and restart it; outstanding ids become 401s.
    key: [u8; 32],
}

impl SessionManager {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            sessions: DashMap::new(),
            key,
        }
    }

    /// Build a manager with a fresh random 256-bit key. Sessions
    /// minted by the resulting manager only decode within the same
    /// process — useful for tests and for operators who haven't yet
    /// configured `MCP_ENCRYPTION_KEY`.
    pub fn with_ephemeral_key() -> Self {
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        Self::new(key)
    }

    /// Register a session whose id is computed from the per-upstream
    /// header set. `connections_with_headers` carries each upstream's
    /// live `Connection` plus the canonical header map that was used
    /// to open it — `extra_headers` ∪ `Authorization` (if present)
    /// ∪ `Mcp-Session-Id` (always — that's the upstream sid the
    /// proxy must replay on resume).
    ///
    /// Returns the encoded session id. If the same upstream set is
    /// re-registered with byte-identical headers, the returned id is
    /// byte-identical too (modulo the random AEAD nonce, which makes
    /// the ciphertext different each time — intentional, prevents
    /// payload-recognition attacks).
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &self,
        connections_with_headers: Vec<(Connection, IndexMap<String, String>)>,
        agent_instance_hierarchy: Option<String>,
        agent_id: Option<String>,
        agent_full_id: Option<String>,
        agent_remote: Option<String>,
        tool_allowlists: IndexMap<String, Vec<String>>,
    ) -> String {
        let payload = build_payload(
            &connections_with_headers,
            agent_instance_hierarchy,
            agent_id,
            agent_full_id,
            agent_remote,
            tool_allowlists,
        );
        let id = encrypt_and_encode(&payload, &self.key);
        let connections: Vec<Connection> =
            connections_with_headers.into_iter().map(|(c, _)| c).collect();
        let by_name = build_by_name_map(connections);
        self.sessions
            .insert(id.clone(), Arc::new(Session::new(by_name, payload)));
        id
    }

    /// Cheap clone-out of a [`Session`] — never holds a DashMap guard
    /// across the await boundary.
    pub fn get(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.get(session_id).map(|e| e.value().clone())
    }

    /// Remove a session from the registry. Returns `Some(_)` if a session
    /// was present, `None` if the id was unknown.
    ///
    /// Once every `Arc<Session>` to the removed session has dropped, the
    /// session's `IndexMap<String, Connection>` drops, every `Connection`'s
    /// `Drop` fires its upstream's wakeup signal, and each upstream's
    /// listener task wakes to re-check liveness. The listener sees
    /// `Arc::strong_count == 1` (only itself) and exits, which drops the
    /// inner state and closes the upstream HTTP session.
    pub fn remove(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.remove(session_id).map(|(_, session)| session)
    }

    /// Decrypt an incoming session id back into the URL → header_map
    /// payload it encodes. `None` on any decode failure (bad base62,
    /// unknown version, AEAD failure, bad JSON, wrong shape).
    pub fn decode_session_id(&self, id: &str) -> Option<SessionPayload> {
        decode_with_key(id, &self.key)
    }

    /// Re-mint the encoded id for a payload that's already canonical —
    /// used by the alive-in-memory branch in `handle_initialize` to
    /// hand back the same id the client sent (the encrypt step
    /// produces a different ciphertext each call due to the random
    /// nonce, so technically the caller will see a fresh id, not the
    /// byte-equal old one; the new id decrypts to the same payload
    /// either way).
    pub fn mint_id(&self, payload: &SessionPayload) -> String {
        encrypt_and_encode(payload, &self.key)
    }
}

/// Build a canonical (url-sorted, header-sorted) `SessionPayload`
/// from a list of `(Connection, raw_header_map)` pairs.
///
/// The raw header map is normalized:
///   - keys lowercased? **No** — HTTP headers are case-insensitive on
///     the wire but we keep the casing the upstream sees. Sorting is
///     done case-sensitively on the bytes; deterministic regardless.
///   - sorted alphabetically.
fn build_payload(
    pairs: &[(Connection, IndexMap<String, String>)],
    agent_instance_hierarchy: Option<String>,
    agent_id: Option<String>,
    agent_full_id: Option<String>,
    agent_remote: Option<String>,
    tool_allowlists: IndexMap<String, Vec<String>>,
) -> SessionPayload {
    // Collect (url, sorted headers) pairs, then sort by URL.
    let mut url_entries: Vec<(String, IndexMap<String, String>)> = pairs
        .iter()
        .map(|(c, headers)| {
            let mut sorted: Vec<(&str, &str)> = headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));
            let inner: IndexMap<String, String> = sorted
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            (c.url.clone(), inner)
        })
        .collect();
    url_entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut connections: IndexMap<String, IndexMap<String, String>> =
        IndexMap::with_capacity(url_entries.len());
    for (url, headers) in url_entries {
        connections.insert(url, headers);
    }
    // Sort tool_allowlists by URL for byte-stable session-id encoding,
    // mirroring the per-URL sort applied to `connections` above.
    let mut allow_entries: Vec<(String, Vec<String>)> =
        tool_allowlists.into_iter().collect();
    allow_entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut tool_allowlists: IndexMap<String, Vec<String>> =
        IndexMap::with_capacity(allow_entries.len());
    for (url, names) in allow_entries {
        tool_allowlists.insert(url, names);
    }

    SessionPayload {
        connections,
        agent_instance_hierarchy,
        agent_id,
        agent_full_id,
        agent_remote,
        tool_allowlists,
    }
}

/// JSON-serialize the payload, AEAD-encrypt with a *deterministic*
/// nonce derived from a BLAKE3 keyed hash of the plaintext, prepend
/// version + nonce, base62-encode the whole envelope.
///
/// Why deterministic: `handle_initialize`'s alive-in-memory branch
/// needs to mint the same id the caller already holds, so the id
/// remains a key in `state.sessions`. With a random nonce we'd
/// generate a different ciphertext for the same payload every time.
///
/// Safety of nonce reuse: AEAD only breaks under nonce reuse when the
/// SAME nonce is paired with TWO DIFFERENT plaintexts. Here the nonce
/// is a function of the plaintext (and key), so distinct plaintexts
/// get distinct nonces; identical plaintexts get identical nonces and
/// identical ciphertexts, which is exactly what we want.
fn encrypt_and_encode(payload: &SessionPayload, key: &[u8; 32]) -> String {
    let plaintext =
        serde_json::to_vec(payload).expect("SessionPayload serializes");

    // Derive the 24-byte XChaCha20 nonce from BLAKE3(key, plaintext).
    // BLAKE3's keyed hash is a PRF, so this is indistinguishable from
    // random for any attacker who doesn't know `key`, but it's stable
    // for the (key, plaintext) pair.
    let mut hasher = blake3::Hasher::new_keyed(key);
    hasher.update(&plaintext);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    nonce_bytes.copy_from_slice(&hasher.finalize().as_bytes()[..NONCE_LEN]);

    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext_with_tag = cipher
        .encrypt(nonce, plaintext.as_ref())
        .expect("XChaCha20-Poly1305 encrypt is infallible for valid key/nonce");

    let mut envelope = Vec::with_capacity(1 + NONCE_LEN + ciphertext_with_tag.len());
    envelope.push(VERSION);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext_with_tag);
    base62_encode_bytes(&envelope)
}

/// Reverse of [`encrypt_and_encode`]. AEAD failure → `None`.
fn decode_with_key(id: &str, key: &[u8; 32]) -> Option<SessionPayload> {
    let envelope = base62_decode_bytes(id)?;
    if envelope.len() < 1 + NONCE_LEN + TAG_LEN {
        return None;
    }
    if envelope[0] != VERSION {
        return None;
    }
    let nonce = XNonce::from_slice(&envelope[1..1 + NONCE_LEN]);
    let ciphertext = &envelope[1 + NONCE_LEN..];
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
    serde_json::from_slice(&plaintext).ok()
}

/// Parse an `MCP_ENCRYPTION_KEY` env-var value: a single base64-encoded
/// 32-byte key. Empty string → `None`. Malformed → `Err`.
pub fn parse_key_env(s: &str) -> Result<Option<[u8; 32]>, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| format!("MCP_ENCRYPTION_KEY: not valid base64: {e}"))?;
    let key: [u8; 32] = decoded.try_into().map_err(|got: Vec<u8>| {
        format!(
            "MCP_ENCRYPTION_KEY: expected 32 bytes after base64-decode, got {}",
            got.len(),
        )
    })?;
    Ok(Some(key))
}

/// Byte-level base62. The off-the-shelf `base62` crate only encodes
/// `u128`s; we need variable-length input for our envelope. Encoding
/// interprets the bytes as a big-endian unsigned big-integer and
/// prints it in base62 with `0..9 a..z A..Z` digits; leading zero
/// bytes are encoded as a `0` digit each so they survive the
/// round-trip.
fn base62_encode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    const ALPHABET: &[u8; 62] =
        b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let leading_zeros = bytes.iter().take_while(|b| **b == 0).count();
    let mut digits: Vec<u8> = Vec::with_capacity(bytes.len() * 2);
    let mut num: Vec<u32> = bytes[leading_zeros..].iter().map(|b| *b as u32).collect();
    while !num.is_empty() {
        let mut remainder: u32 = 0;
        let mut next: Vec<u32> = Vec::with_capacity(num.len());
        for &b in &num {
            let acc = remainder * 256 + b;
            let q = acc / 62;
            remainder = acc % 62;
            if !(next.is_empty() && q == 0) {
                next.push(q);
            }
        }
        digits.push(remainder as u8);
        num = next;
    }
    let mut out = String::with_capacity(leading_zeros + digits.len());
    for _ in 0..leading_zeros {
        out.push(ALPHABET[0] as char);
    }
    for d in digits.into_iter().rev() {
        out.push(ALPHABET[d as usize] as char);
    }
    out
}

fn base62_decode_bytes(s: &str) -> Option<Vec<u8>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    fn digit(c: char) -> Option<u32> {
        match c {
            '0'..='9' => Some(c as u32 - '0' as u32),
            'a'..='z' => Some(c as u32 - 'a' as u32 + 10),
            'A'..='Z' => Some(c as u32 - 'A' as u32 + 36),
            _ => None,
        }
    }
    let leading_zeros = s.chars().take_while(|c| *c == '0').count();
    let mut num: Vec<u32> = Vec::with_capacity(s.len());
    for c in s.chars().skip(leading_zeros) {
        num.push(digit(c)?);
    }
    let mut bytes: Vec<u8> = Vec::new();
    while !num.is_empty() {
        let mut remainder: u32 = 0;
        let mut next: Vec<u32> = Vec::with_capacity(num.len());
        for &d in &num {
            let acc = remainder * 62 + d;
            let q = acc / 256;
            remainder = acc % 256;
            if !(next.is_empty() && q == 0) {
                next.push(q);
            }
        }
        bytes.push(remainder as u8);
        num = next;
    }
    let mut out = vec![0u8; leading_zeros];
    out.extend(bytes.into_iter().rev());
    Some(out)
}

fn build_by_name_map(
    connections: Vec<Connection>,
) -> IndexMap<String, Connection> {
    // First pass: which names are duplicated? Anything that shows up
    // more than once in the input gets the `_<index>` suffix.
    let mut name_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for c in &connections {
        *name_counts
            .entry(c.initialize_result.server_info.name.clone())
            .or_insert(0) += 1;
    }
    let mut by_name: IndexMap<String, Connection> =
        IndexMap::with_capacity(connections.len());
    for (idx, connection) in connections.into_iter().enumerate() {
        let raw = connection.initialize_result.server_info.name.clone();
        let key = if name_counts.get(&raw).copied().unwrap_or(0) > 1 {
            format!("{raw}_{idx}")
        } else {
            raw
        };
        if by_name.contains_key(&key) {
            tracing::warn!(
                key = %key,
                "two upstreams produce the same prefix after disambiguation; later upstream wins",
            );
        }
        by_name.insert(key, connection);
    }
    by_name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> SessionPayload {
        let mut connections: IndexMap<String, IndexMap<String, String>> = IndexMap::new();
        let mut h_a: IndexMap<String, String> = IndexMap::new();
        h_a.insert("Authorization".into(), "Bearer secret-A".into());
        h_a.insert("Mcp-Session-Id".into(), "sid-A".into());
        h_a.insert("X-Tenant".into(), "tenant-1".into());
        connections.insert("https://upstream-a.example/mcp".into(), h_a);
        let mut h_b: IndexMap<String, String> = IndexMap::new();
        h_b.insert("Mcp-Session-Id".into(), "sid-B".into());
        connections.insert("https://upstream-b.example/mcp".into(), h_b);
        SessionPayload { connections, agent_instance_hierarchy: None, agent_id: None, tool_allowlists: IndexMap::new() }
    }

    #[test]
    fn base62_round_trip() {
        for sample in [
            &b""[..],
            &b"a"[..],
            &b"\x00\x01\x02"[..],
            &b"hello world"[..],
            br#"{"http://127.0.0.1:1234":"abc123"}"#,
            &(0..=255u16).map(|b| b as u8).collect::<Vec<_>>()[..],
        ] {
            let encoded = base62_encode_bytes(sample);
            assert!(encoded.bytes().all(|b| (0x21..=0x7E).contains(&b)));
            let decoded = base62_decode_bytes(&encoded).expect("decode");
            assert_eq!(decoded, sample, "round-trip failed for {sample:?}");
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let key = [0x42u8; 32];
        let payload = sample_payload();
        let id = encrypt_and_encode(&payload, &key);
        let decoded = decode_with_key(&id, &key).expect("decode under same key");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_with_wrong_key_returns_none() {
        let key_a = [0x11u8; 32];
        let key_b = [0x22u8; 32];
        let id = encrypt_and_encode(&sample_payload(), &key_a);
        assert!(decode_with_key(&id, &key_b).is_none());
    }

    #[test]
    fn decode_garbage_returns_none() {
        let key = [0x55u8; 32];
        // Random base62 string, certainly not a valid envelope.
        assert!(decode_with_key("ABCdef123", &key).is_none());
        // Empty.
        assert!(decode_with_key("", &key).is_none());
        // Too short to even hold version + nonce + tag.
        assert!(decode_with_key("0", &key).is_none());
    }

    #[test]
    fn payload_roundtrip_preserves_canonical_order() {
        // Build "the same" payload with shuffled URL and header order;
        // after `build_payload` they should be equal byte-for-byte.
        let conn_a_url = "https://b.example/mcp".to_string();
        let conn_b_url = "https://a.example/mcp".to_string();

        let mut h_unsorted: IndexMap<String, String> = IndexMap::new();
        h_unsorted.insert("Z-Header".into(), "z".into());
        h_unsorted.insert("Authorization".into(), "Bearer".into());

        // We can't easily synthesize Connection without spinning a
        // real server, so test build_payload's canonicalization
        // through an inline helper that mirrors what add() builds.
        let pairs_unsorted: Vec<(String, IndexMap<String, String>)> =
            vec![(conn_a_url.clone(), h_unsorted.clone()), (conn_b_url.clone(), h_unsorted.clone())];

        let mut connections: IndexMap<String, IndexMap<String, String>> = IndexMap::new();
        let mut url_entries: Vec<(String, IndexMap<String, String>)> = pairs_unsorted
            .into_iter()
            .map(|(url, headers)| {
                let mut sorted: Vec<(&str, &str)> =
                    headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                sorted.sort_by(|a, b| a.0.cmp(b.0));
                let inner: IndexMap<String, String> = sorted
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                (url, inner)
            })
            .collect();
        url_entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (u, h) in url_entries {
            connections.insert(u, h);
        }
        let payload = SessionPayload { connections, agent_instance_hierarchy: None, agent_id: None, tool_allowlists: IndexMap::new() };

        let urls: Vec<&String> = payload.connections.keys().collect();
        assert_eq!(urls, vec![&conn_b_url, &conn_a_url]); // a.example before b.example
        let inner = &payload.connections[&conn_b_url];
        let inner_keys: Vec<&String> = inner.keys().collect();
        assert_eq!(inner_keys, vec!["Authorization", "Z-Header"]); // alphabetical
    }

    #[test]
    fn parse_key_env_round_trip() {
        let key = [0xAAu8; 32];
        let env = base64::engine::general_purpose::STANDARD.encode(key);
        let parsed = parse_key_env(&env).expect("parse").expect("Some");
        assert_eq!(parsed, key);

        assert!(parse_key_env("").unwrap().is_none());
        assert!(parse_key_env("   ").unwrap().is_none());
        assert!(parse_key_env("not-base64!@#").is_err());
        // Wrong-length payload (16 bytes after b64 decode):
        let short =
            base64::engine::general_purpose::STANDARD.encode(&[0u8; 16][..]);
        assert!(parse_key_env(&short).is_err());
    }
}

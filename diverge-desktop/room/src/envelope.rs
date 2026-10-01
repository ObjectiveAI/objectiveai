//! Words sealed to someone: what a visitor leaves on a profile stays
//! between them and the profile's owner.
//!
//! - **The owner's notes key.** An X25519 key drawn (HKDF) from the
//!   account's recovery words on a path of its own ([`NOTES_PATH`]). Its
//!   public half is named in the profile room's signed settings
//!   (`Args::notes_key`), so a visitor seals to the key the owner signed.
//! - **An author's key.** An X25519 key drawn from the key that seals the
//!   call, for one room ([`OpenKey::for_author`]): whoever wrote something
//!   can open it again from any copy, and a hire's result can be sealed back
//!   to whoever asked.
//! - **An envelope.** The words are sealed once (ChaCha20-Poly1305) under a
//!   fresh key, and that key is sealed to each reader through a fresh
//!   X25519 exchange. The room, the verb and the key that seals the call are
//!   bound in, so an envelope can't be carried to another room, verb or
//!   author. Nobody can tell from an envelope who its readers are.
//!
//! A room holding envelopes holds ciphertext: every copy of its record does
//! too. The room checks only an envelope's shape; it can't read one.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key as AeadKey, Nonce};
use curve25519_dalek::montgomery::MontgomeryPoint;
use hkdf::Hkdf;
use rmcp::model::JsonObject;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use crate::seal::{Keypair, canonical};

/// The envelope format this program writes and reads.
pub const VERSION: u32 = 1;

/// The most readers one envelope is sealed to.
pub const MOST_READERS: usize = 4;

/// The most sealed bytes one envelope carries: a hire's whole result fits.
pub const MOST_BYTES: usize = 256 * 1024;

/// Where the notes key sits under the recovery words: its own HKDF path,
/// apart from the account's root (SLIP-10, `m/0'`).
pub const NOTES_PATH: &str = "diverge-desktop notes key m/notes/0";

/// The verbs whose words a profile room with a notes key takes only sealed.
pub const SEALED_VERBS: [&str; 4] = ["leave_note", "hire", "answer_hire", "deliver_hire"];

/// What stays in the clear for a sealed verb: only what the room needs to
/// keep its state (which hire, and whether it was taken).
pub fn clear_keys(verb: &str) -> &'static [&'static str] {
    match verb {
        "answer_hire" => &["hire_id", "take"],
        "deliver_hire" => &["hire_id"],
        _ => &[],
    }
}

/// The verb that made a move of this kind, for the sealed verbs.
pub fn verb_of_kind(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "note" => "leave_note",
        "hire" => "hire",
        "hire_answer" => "answer_hire",
        "hire_delivery" => "deliver_hire",
        _ => return None,
    })
}

/// An X25519 secret that opens envelopes: an owner's notes key, or an
/// author's key for one room. Wiped when dropped.
pub struct OpenKey([u8; 32]);

impl Drop for OpenKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

fn draw(salt: &[u8], ikm: &[u8], info: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    Hkdf::<Sha256>::new(Some(salt), ikm).expand(info, &mut out).expect("32 bytes is a length HKDF gives");
    out
}

impl OpenKey {
    /// The owner's notes key, from the recovery words' seed.
    pub fn from_seed(seed: &[u8]) -> Self {
        OpenKey(draw(b"diverge-desktop notes key", seed, NOTES_PATH.as_bytes()))
    }

    /// The key whoever seals a call with `keypair` opens their own words
    /// with, in one room. Drawn from their signing key: nothing new to keep.
    pub fn for_author(keypair: &Keypair, room: &str) -> Self {
        let secret = Zeroizing::new(hex::decode(Zeroizing::new(keypair.secret_hex()).as_str()).expect("a key's secret is hex"));
        OpenKey(draw(room.as_bytes(), &secret, b"diverge-desktop author key"))
    }

    /// A new key nobody else holds.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("the system has randomness");
        OpenKey(bytes)
    }

    /// Its public half, as hex: what others seal to.
    pub fn public(&self) -> String {
        hex::encode(MontgomeryPoint::mul_base_clamped(self.0).to_bytes())
    }
}

/// Words sealed to a few readers. See the module's notes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    /// The sealer's one-time X25519 public key.
    pub eph: String,
    /// The words' key, sealed to each reader in turn.
    pub to: Vec<String>,
    pub nonce: String,
    /// The words, sealed.
    pub sealed: String,
}

fn point(hex_key: &str) -> Result<MontgomeryPoint, String> {
    let bytes: [u8; 32] = hex::decode(hex_key).map_err(|_| "a key is hex".to_string())?.try_into().map_err(|_| "a key is 32 bytes".to_string())?;
    Ok(MontgomeryPoint(bytes))
}

/// What an envelope is bound to: its room, the verb it rides in, and the key that seals that call.
fn bound(room: &str, verb: &str, author: &str) -> Vec<u8> {
    canonical(&json!({ "sealed words": VERSION, "room": room, "verb": verb, "author": author })).into_bytes()
}

/// The key that seals the words' key to one reader.
fn wrap_key(shared: &MontgomeryPoint, eph: &MontgomeryPoint, reader: &MontgomeryPoint) -> Result<Zeroizing<[u8; 32]>, String> {
    if shared.to_bytes() == [0u8; 32] {
        return Err("that reader's key can't be sealed to".into());
    }
    let mut salt = [0u8; 64];
    salt[..32].copy_from_slice(eph.as_bytes());
    salt[32..].copy_from_slice(reader.as_bytes());
    Ok(Zeroizing::new(draw(&salt, shared.as_bytes(), b"diverge-desktop sealed words key")))
}

/// Seal `words` to `readers` (X25519 public keys, hex), for one room, verb and author.
pub fn seal(words: &Value, readers: &[String], room: &str, verb: &str, author: &str) -> Result<Envelope, String> {
    if readers.is_empty() || readers.len() > MOST_READERS {
        return Err(format!("sealed words go to between 1 and {MOST_READERS} readers"));
    }
    let aad = bound(room, verb, author);
    let mut words_key = Zeroizing::new([0u8; 32]);
    let mut nonce = [0u8; 12];
    getrandom::getrandom(words_key.as_mut()).expect("the system has randomness");
    getrandom::getrandom(&mut nonce).expect("the system has randomness");
    let plain = Zeroizing::new(canonical(words).into_bytes());
    let sealed = ChaCha20Poly1305::new(AeadKey::from_slice(words_key.as_ref())).encrypt(Nonce::from_slice(&nonce), Payload { msg: &plain, aad: &aad }).map_err(|_| "the words wouldn't seal".to_string())?;
    if sealed.len() > MOST_BYTES {
        return Err("those words are too long to seal here".into());
    }
    let eph_secret = OpenKey::generate();
    let eph = MontgomeryPoint::mul_base_clamped(eph_secret.0);
    let mut to = Vec::new();
    for reader in readers {
        let r = point(reader)?;
        let key = wrap_key(&r.mul_clamped(eph_secret.0), &eph, &r)?;
        // Each wrap key is used once, so a zero nonce is safe.
        let wrapped = ChaCha20Poly1305::new(AeadKey::from_slice(key.as_ref())).encrypt(Nonce::from_slice(&[0u8; 12]), Payload { msg: words_key.as_ref(), aad: &aad }).map_err(|_| "the words' key wouldn't seal".to_string())?;
        to.push(hex::encode(wrapped));
    }
    Ok(Envelope { v: VERSION, eph: hex::encode(eph.to_bytes()), to, nonce: hex::encode(nonce), sealed: hex::encode(sealed) })
}

/// Open an envelope with one key, if it was sealed to it, for this room, verb and author.
pub fn open(envelope: &Envelope, key: &OpenKey, room: &str, verb: &str, author: &str) -> Option<Value> {
    if envelope.v != VERSION {
        return None;
    }
    let aad = bound(room, verb, author);
    let eph = point(&envelope.eph).ok()?;
    let me = MontgomeryPoint::mul_base_clamped(key.0);
    let wrap = wrap_key(&eph.mul_clamped(key.0), &eph, &me).ok()?;
    let cipher = ChaCha20Poly1305::new(AeadKey::from_slice(wrap.as_ref()));
    let words_key = envelope.to.iter().find_map(|t| {
        let wrapped = hex::decode(t).ok()?;
        cipher.decrypt(Nonce::from_slice(&[0u8; 12]), Payload { msg: &wrapped, aad: &aad }).ok().map(Zeroizing::new)
    })?;
    if words_key.len() != 32 {
        return None;
    }
    let nonce = hex::decode(&envelope.nonce).ok().filter(|n| n.len() == 12)?;
    let sealed = hex::decode(&envelope.sealed).ok()?;
    let plain = Zeroizing::new(ChaCha20Poly1305::new(AeadKey::from_slice(&words_key)).decrypt(Nonce::from_slice(&nonce), Payload { msg: &sealed, aad: &aad }).ok()?);
    serde_json::from_slice(&plain).ok()
}

/// Open with whichever of `keys` it was sealed to.
pub fn open_with(envelope: &Envelope, keys: &[OpenKey], room: &str, verb: &str, author: &str) -> Option<Value> {
    keys.iter().find_map(|k| open(envelope, k, room, verb, author))
}

/// What a room checks of an envelope it can't read: its shape.
pub fn shape(value: &Value) -> Result<Envelope, String> {
    let e: Envelope = serde_json::from_value(value.clone()).map_err(|_| "sealed words come in an envelope: v, eph, to, nonce, sealed".to_string())?;
    let hex_of = |s: &str, len: Option<usize>| hex::decode(s).ok().filter(|b| len.is_none_or(|l| b.len() == l)).is_some();
    if e.v != VERSION {
        return Err(format!("this program reads sealed words of version {VERSION} only"));
    }
    if e.to.is_empty() || e.to.len() > MOST_READERS {
        return Err(format!("sealed words go to between 1 and {MOST_READERS} readers"));
    }
    // A wrapped key is the 32-byte key and its 16-byte tag.
    if !hex_of(&e.eph, Some(32)) || !hex_of(&e.nonce, Some(12)) || !e.to.iter().all(|t| hex_of(t, Some(48))) {
        return Err("that envelope is damaged".into());
    }
    let sealed = hex::decode(&e.sealed).map_err(|_| "that envelope is damaged".to_string())?;
    if sealed.len() < 16 || sealed.len() > MOST_BYTES {
        return Err("that envelope is damaged or too long".into());
    }
    Ok(e)
}

/// A sealed verb's arguments, as they travel: what stays in the clear for
/// it, and the rest sealed under `sealed` to `readers`. Arguments that
/// already carry an envelope travel as they are; a verb that isn't sealed,
/// or one with nothing left to seal, travels as it is too.
pub fn seal_args(room: &str, verb: &str, args: &JsonObject, author: &str, readers: &[String]) -> Result<JsonObject, String> {
    if !SEALED_VERBS.contains(&verb) || args.contains_key("sealed") {
        return Ok(args.clone());
    }
    let clear = clear_keys(verb);
    let mut out = JsonObject::new();
    let mut words = Map::new();
    for (k, v) in args {
        if clear.contains(&k.as_str()) {
            out.insert(k.clone(), v.clone());
        } else if !v.is_null() && v.as_str() != Some("") {
            words.insert(k.clone(), v.clone());
        }
    }
    if !words.is_empty() {
        let envelope = seal(&Value::Object(words), readers, room, verb, author)?;
        out.insert("sealed".into(), serde_json::to_value(envelope).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_words_open_only_for_their_readers_room_verb_and_author() {
        let owner = OpenKey::from_seed(b"an account's seed");
        let ren = Keypair::from_seed("ren");
        let ren_reads = OpenKey::for_author(&ren, "profile-1");
        let ada = OpenKey::for_author(&Keypair::from_seed("ada"), "profile-1");
        let words = json!({ "body": "love the lamp" });
        let e = seal(&words, &[owner.public(), ren_reads.public()], "profile-1", "leave_note", &ren.key()).unwrap();
        assert_eq!(open(&e, &owner, "profile-1", "leave_note", &ren.key()), Some(words.clone()), "the owner reads it");
        assert_eq!(open(&e, &ren_reads, "profile-1", "leave_note", &ren.key()), Some(words.clone()), "its author reads it");
        assert_eq!(open(&e, &ada, "profile-1", "leave_note", &ren.key()), None, "nobody else does");
        assert_eq!(open(&e, &owner, "profile-2", "leave_note", &ren.key()), None, "not carried to another room");
        assert_eq!(open(&e, &owner, "profile-1", "hire", &ren.key()), None, "nor another verb");
        assert_eq!(open(&e, &owner, "profile-1", "leave_note", &Keypair::from_seed("ada").key()), None, "nor passed off as someone else's");
        let text = serde_json::to_string(&e).unwrap();
        assert!(!text.contains("lamp") && !text.contains(&hex::encode("love the lamp")), "the envelope holds no words");
        assert!(!text.contains(&owner.public()) && !text.contains(&ren_reads.public()), "nor who can read it");
        // Changed in any part, it doesn't open.
        let mut changed = e.clone();
        let mut bytes = hex::decode(&changed.sealed).unwrap();
        bytes[0] ^= 1;
        changed.sealed = hex::encode(bytes);
        assert_eq!(open(&changed, &owner, "profile-1", "leave_note", &ren.key()), None);
        assert!(shape(&serde_json::to_value(&e).unwrap()).is_ok());
        assert!(shape(&json!({ "body": "plain" })).is_err());
    }

    #[test]
    fn the_notes_key_and_an_authors_key_are_their_own() {
        assert_eq!(OpenKey::from_seed(b"seed").public(), OpenKey::from_seed(b"seed").public(), "the same words draw the same key");
        assert_ne!(OpenKey::from_seed(b"seed").public(), OpenKey::from_seed(b"other").public());
        let ren = Keypair::from_seed("ren");
        assert_ne!(OpenKey::for_author(&ren, "a").public(), OpenKey::for_author(&ren, "b").public(), "one per room, so nothing links rooms");
        assert_ne!(OpenKey::for_author(&ren, "a").public(), ren.key());
    }

    #[test]
    fn a_sealed_verb_keeps_only_what_the_room_needs_in_the_clear() {
        let owner = OpenKey::generate();
        let args = json!({ "hire_id": "hire-3", "take": true, "note": "Saturday" }).as_object().cloned().unwrap();
        let out = seal_args("p", "answer_hire", &args, "k", &[owner.public()]).unwrap();
        let mut keys: Vec<&String> = out.keys().collect();
        keys.sort();
        assert_eq!(keys, ["hire_id", "sealed", "take"]);
        let e = shape(&out["sealed"]).unwrap();
        assert_eq!(open(&e, &owner, "p", "answer_hire", "k"), Some(json!({ "note": "Saturday" })));
        let bare = json!({ "hire_id": "hire-3", "take": false }).as_object().cloned().unwrap();
        assert_eq!(seal_args("p", "answer_hire", &bare, "k", &[owner.public()]).unwrap(), bare, "nothing to seal");
        let show = json!({ "title": "x" }).as_object().cloned().unwrap();
        assert_eq!(seal_args("p", "show", &show, "k", &[owner.public()]).unwrap(), show, "not a sealed verb");
    }
}

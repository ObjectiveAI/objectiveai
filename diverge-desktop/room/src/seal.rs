//! Seals: how a room knows who said what, when the wire can't tell it.
//!
//! A tool container's program sees one MCP client, its proxy, and a call
//! carries no member. The runner learns only an address and the opaque
//! string a joiner wrote. So each person (and each agent) has a key their
//! own Mac makes and keeps. Every call they make carries a seal under one
//! `_meta` key of ours: a signature over the room, the verb, its arguments
//! and a counter that only goes up. `_meta` passes through the provider and
//! the proxy untouched, so the room program can check the seal itself.
//!
//! A [`Statement`] is the same idea for a fact rather than a call: an
//! agent's key tethered to its person, a vouch, a receipt, a settlement.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rmcp::model::{CallToolRequestParams, JsonObject, RequestMetaObject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The `_meta` key a seal rides under.
pub const META_SEAL: &str = "network.diverge.desktop/seal";

/// A public key, as hex. What a room, a member or a record names someone by.
pub type Key = String;

/// A signing key. Never leaves the Mac that made it.
#[derive(Clone)]
pub struct Keypair(SigningKey);

impl Keypair {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).expect("the system has randomness");
        Keypair(SigningKey::from_bytes(&bytes))
    }

    /// For invented people in the stand-in only: the same key every run.
    pub fn from_seed(seed: &str) -> Self {
        let bytes: [u8; 32] = Sha256::digest(format!("diverge-desktop stand-in: {seed}").as_bytes()).into();
        Keypair(SigningKey::from_bytes(&bytes))
    }

    pub fn from_secret_hex(secret: &str) -> Result<Self, String> {
        let bytes: [u8; 32] = hex::decode(secret).map_err(|e| e.to_string())?.try_into().map_err(|_| "a secret is 32 bytes".to_string())?;
        Ok(Keypair(SigningKey::from_bytes(&bytes)))
    }

    /// A key from its 32 secret bytes: an account's root, derived from its words.
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        Keypair(SigningKey::from_bytes(bytes))
    }

    pub fn secret_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    pub fn key(&self) -> Key {
        hex::encode(self.0.verifying_key().to_bytes())
    }

    fn sign(&self, bytes: &[u8]) -> String {
        hex::encode(self.0.sign(bytes).to_bytes())
    }

    /// A room's countersign on one of its moves.
    pub fn countersign(&self, hash: &str) -> String {
        self.sign(&countersign_bytes(hash))
    }
}

fn countersign_bytes(hash: &str) -> Vec<u8> {
    canonical(&json!({ "countersign": hash })).into_bytes()
}

/// Whether a room's key countersigned a move's hash.
pub fn countersigned(room_key: &str, hash: &str, sig: &str) -> bool {
    verify(room_key, &countersign_bytes(hash), sig)
}

/// The part of a room's id that names its host: the start of their key's digest.
fn host_mark(host_key: &str) -> String {
    digest(host_key.as_bytes())[..12].to_owned()
}

/// A room's id: a label its host chose, then the host's mark. No other host
/// can make a room with the same id, and every seal names the id, so a seal
/// made for one room is refused in every other.
pub fn room_id(label: &str, host_key: &str) -> String {
    format!("{label}.{}", host_mark(host_key))
}

/// Whether an id is one its host could have made: a plain label, then their mark.
pub fn id_holds(id: &str, host_key: &str) -> bool {
    match id.rsplit_once('.') {
        Some((label, mark)) => !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') && mark == host_mark(host_key),
        None => false,
    }
}

/// The part of a room's id that names its host's account: the start of a
/// digest of the account's id, kept apart from a key's mark so neither can
/// stand for the other.
fn account_mark(account: &str) -> String {
    digest(format!("diverge-desktop account\n{account}").as_bytes())[..12].to_owned()
}

/// A room's id under rules 2: a label, then the host's account's mark. Any
/// device of that account hosts the same id.
pub fn account_room_id(label: &str, account: &str) -> String {
    format!("{label}.{}", account_mark(account))
}

/// Whether an id is one an account could have made: a plain label, then its mark.
pub fn account_id_holds(id: &str, account: &str) -> bool {
    match id.rsplit_once('.') {
        Some((label, mark)) => !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') && mark == account_mark(account),
        None => false,
    }
}

/// A label nobody else will pick: for a new room's id.
pub fn fresh_label() -> String {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("the system has randomness");
    hex::encode(bytes)
}

fn verify(key: &str, bytes: &[u8], sig: &str) -> bool {
    let Ok(key) = hex::decode(key).map_err(|_| ()).and_then(|b| <[u8; 32]>::try_from(b).map_err(|_| ())) else { return false };
    let Ok(key) = VerifyingKey::from_bytes(&key) else { return false };
    let Ok(sig) = hex::decode(sig).map_err(|_| ()).and_then(|b| <[u8; 64]>::try_from(b).map_err(|_| ())) else { return false };
    key.verify(bytes, &Signature::from_bytes(&sig)).is_ok()
}

/// JSON with every object's keys sorted, so two parties seal the same bytes.
pub fn canonical(value: &Value) -> String {
    fn sort(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    out.insert(k.clone(), sort(&map[k]));
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(sort).collect()),
            other => other.clone(),
        }
    }
    sort(value).to_string()
}

/// The whole SHA-256 of some bytes, as hex.
pub fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A short, stable name for a text: a charter's version.
pub fn fingerprint(text: &str) -> String {
    digest(text.as_bytes())[..16].to_owned()
}

/// A call's seal.
///
/// It carries two signatures by the same key. `sig` is over the arguments
/// themselves: what a room under rules 1 checks and keeps. `words_sig` is
/// over `words`, a digest of the arguments salted with `salt`: what a room
/// under rules 2 checks and chains, so the arguments (and the salt) can be
/// erased later while the seal still holds. Each room keeps only the form
/// its rules use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    pub key: Key,
    pub counter: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sig: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub words_sig: Option<String>,
}

impl Seal {
    /// The seal as a room under rules 1 keeps it: over the arguments, nothing else.
    pub fn whole(&self) -> Seal {
        Seal { key: self.key.clone(), counter: self.counter, sig: self.sig.clone(), words: None, salt: None, words_sig: None }
    }

    /// The seal as a room under rules 2 keeps it: over the salted digest, never the arguments.
    pub fn by_words(&self) -> Seal {
        Seal { sig: String::new(), ..self.clone() }
    }

    /// Whether this is the form sealed over a digest of the words.
    pub fn is_by_words(&self) -> bool {
        self.words.is_some()
    }
}

fn call_bytes(room: &str, verb: &str, args: &JsonObject, counter: u64) -> Vec<u8> {
    canonical(&json!({ "room": room, "verb": verb, "arguments": Value::Object(args.clone()), "counter": counter })).into_bytes()
}

fn words_bytes(room: &str, verb: &str, words: &str, counter: u64) -> Vec<u8> {
    canonical(&json!({ "room": room, "verb": verb, "words": words, "counter": counter })).into_bytes()
}

/// The salted digest of a call's arguments that a rules-2 seal signs.
pub fn words_of(salt: &str, args: &JsonObject) -> String {
    digest(canonical(&json!({ "salt": salt, "arguments": Value::Object(args.clone()) })).as_bytes())
}

/// A fresh salt for one call's words.
fn fresh_salt() -> String {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("the system has randomness");
    hex::encode(bytes)
}

/// Seal a call for one room. The arguments are sealed as they are now:
/// change them afterwards and the seal no longer matches. Both forms are
/// sealed (see [`Seal`]); the room keeps the one its rules use.
pub fn seal_call(keypair: &Keypair, room: &str, params: &mut CallToolRequestParams, counter: u64) {
    let args = params.arguments.clone().unwrap_or_default();
    let salt = fresh_salt();
    let words = words_of(&salt, &args);
    let seal = Seal {
        key: keypair.key(),
        counter,
        sig: keypair.sign(&call_bytes(room, &params.name, &args, counter)),
        words_sig: Some(keypair.sign(&words_bytes(room, &params.name, &words, counter))),
        words: Some(words),
        salt: Some(salt),
    };
    let mut meta = params.meta.take().map(|m| m.0.0).unwrap_or_default();
    meta.insert(META_SEAL.into(), serde_json::to_value(&seal).unwrap_or_default());
    params.meta = Some(RequestMetaObject::from(meta));
}

/// The seal a call carries, if any.
pub fn seal_of(params: &CallToolRequestParams) -> Option<Seal> {
    params.meta.as_ref().and_then(|m| m.0.0.get(META_SEAL)).and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Check a call's seal against the room it was sent to.
pub fn check_call(room: &str, params: &CallToolRequestParams) -> Result<Seal, String> {
    let seal = seal_of(params).ok_or("this call carries no seal")?;
    let args = params.arguments.clone().unwrap_or_default();
    if verify(&seal.key, &call_bytes(room, &params.name, &args, seal.counter), &seal.sig) {
        Ok(seal)
    } else {
        Err("this call's seal does not match it".into())
    }
}

/// Check a call's seal over the digest of its words: the words, salted,
/// make the digest, and the key signed that digest for this room and verb.
pub fn check_words(room: &str, params: &CallToolRequestParams) -> Result<Seal, String> {
    let seal = seal_of(params).ok_or("this call carries no seal")?;
    let args = params.arguments.clone().unwrap_or_default();
    if recheck_words(room, &params.name, Some(&args), &seal) {
        Ok(seal)
    } else {
        Err("this call's seal does not match it".into())
    }
}

/// Re-check a seal kept in a record, against the verb and arguments kept with it.
pub fn recheck(room: &str, verb: &str, args: &JsonObject, seal: &Seal) -> bool {
    verify(&seal.key, &call_bytes(room, verb, args, seal.counter), &seal.sig)
}

/// Re-check a seal over a digest of the words. With the words (`args`), the
/// digest must be theirs under the seal's salt; without them (erased), only
/// the signature over the digest is checked.
pub fn recheck_words(room: &str, verb: &str, args: Option<&JsonObject>, seal: &Seal) -> bool {
    let (Some(words), Some(sig)) = (&seal.words, &seal.words_sig) else { return false };
    if let Some(args) = args {
        match &seal.salt {
            Some(salt) if words_of(salt, args) == *words => {}
            _ => return false,
        }
    }
    verify(&seal.key, &words_bytes(room, verb, words, seal.counter), sig)
}

/// A fact one key vouches for: `tether`, `vouch`, `receipt`, `settled`…
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub kind: String,
    pub body: Value,
    pub key: Key,
    pub sig: String,
}

fn statement_bytes(kind: &str, body: &Value) -> Vec<u8> {
    canonical(&json!({ "statement": kind, "body": body })).into_bytes()
}

/// Whether `sig` is `key`'s statement of `kind` over `body`: for signatures
/// kept apart from their body, such as a room's settings.
pub fn statement_holds(key: &str, kind: &str, body: &Value, sig: &str) -> bool {
    verify(key, &statement_bytes(kind, body), sig)
}

impl Statement {
    pub fn make(keypair: &Keypair, kind: &str, body: Value) -> Self {
        let sig = keypair.sign(&statement_bytes(kind, &body));
        Statement { kind: kind.into(), body, key: keypair.key(), sig }
    }

    pub fn holds(&self) -> bool {
        verify(&self.key, &statement_bytes(&self.kind, &self.body), &self.sig)
    }

    pub fn field(&self, name: &str) -> Option<&str> {
        self.body.get(name).and_then(Value::as_str)
    }
}

/// An agent's key, tethered to the person who runs it.
pub fn tether(person: &Keypair, agent_key: &str, agent_name: &str) -> Statement {
    Statement::make(person, "tether", json!({ "agent": agent_key, "name": agent_name }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seal_holds_only_for_its_room_verb_and_arguments() {
        let ren = Keypair::generate();
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "hi" }).as_object().cloned().unwrap());
        seal_call(&ren, "room-1", &mut params, 1);
        assert_eq!(check_call("room-1", &params).unwrap().key, ren.key());
        assert!(check_call("room-2", &params).is_err(), "not another room");
        let mut changed = params.clone();
        changed.arguments = Some(json!({ "title": "bye" }).as_object().cloned().unwrap());
        assert!(check_call("room-1", &changed).is_err(), "not other words");
        let mut renamed = params.clone();
        renamed.name = "ask".into();
        assert!(check_call("room-1", &renamed).is_err(), "not another verb");
    }

    #[test]
    fn a_seal_over_the_words_holds_without_them_and_only_for_them() {
        let ren = Keypair::generate();
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "hi" }).as_object().cloned().unwrap());
        seal_call(&ren, "room-1", &mut params, 1);
        let seal = check_words("room-1", &params).unwrap();
        assert!(check_words("room-2", &params).is_err(), "not another room");
        let mut changed = params.clone();
        changed.arguments = Some(json!({ "title": "bye" }).as_object().cloned().unwrap());
        assert!(check_words("room-1", &changed).is_err(), "not other words");
        // Erased: the words and the salt are gone, and the signature over the digest still holds.
        let erased = Seal { salt: None, ..seal.by_words() };
        assert!(recheck_words("room-1", "show", None, &erased));
        assert!(!recheck_words("room-1", "ask", None, &erased), "not another verb");
        let mut forged = erased.clone();
        forged.words = Some(digest(b"something else"));
        assert!(!recheck_words("room-1", "show", None, &forged));
        // Rules 1 keeps today's seal, byte for byte.
        let whole = serde_json::to_value(seal.whole()).unwrap();
        let mut keys: Vec<_> = whole.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["counter", "key", "sig"]);
    }

    #[test]
    fn a_room_id_belongs_to_one_host() {
        let (juno, ren) = (Keypair::from_seed("juno"), Keypair::from_seed("ren"));
        let id = room_id("workshop", &juno.key());
        assert!(id_holds(&id, &juno.key()));
        assert!(!id_holds(&id, &ren.key()), "ren can't host juno's id");
        assert!(!id_holds("workshop", &juno.key()), "a bare label names no host");
        assert!(!id_holds(&room_id("../x", &juno.key()), &juno.key()), "a label is plain");
        // An account's room id is its own: a key's mark never stands for an account's, nor the other way.
        let account = digest(b"an account's genesis");
        let theirs = account_room_id("workshop", &account);
        assert!(account_id_holds(&theirs, &account));
        assert!(!account_id_holds(&theirs, &digest(b"another genesis")));
        assert!(!id_holds(&theirs, &account), "an account's id isn't a key's");
        assert!(!account_id_holds(&room_id("workshop", &juno.key()), &juno.key()), "nor a key's an account's");
        let hash = digest(b"a move");
        assert!(countersigned(&juno.key(), &hash, &juno.countersign(&hash)));
        assert!(!countersigned(&ren.key(), &hash, &juno.countersign(&hash)));
        let body = json!({ "title": "x" });
        assert!(statement_holds(&juno.key(), "room", &body, &Statement::make(&juno, "room", body.clone()).sig));
        assert!(!statement_holds(&juno.key(), "vouch", &body, &Statement::make(&juno, "room", body.clone()).sig), "kinds don't cross");
    }

    #[test]
    fn statements_and_keys_round_trip() {
        let juno = Keypair::from_seed("juno");
        assert_eq!(juno.key(), Keypair::from_seed("juno").key(), "stand-in keys are stable");
        assert_eq!(Keypair::from_secret_hex(&juno.secret_hex()).unwrap().key(), juno.key());
        let t = tether(&juno, "abc", "site-fixes");
        assert!(t.holds());
        let mut forged = t.clone();
        forged.body = json!({ "agent": "xyz", "name": "site-fixes" });
        assert!(!forged.holds());
        assert_eq!(canonical(&json!({ "b": 1, "a": { "d": 2, "c": 3 } })), r#"{"a":{"c":3,"d":2},"b":1}"#);
    }
}

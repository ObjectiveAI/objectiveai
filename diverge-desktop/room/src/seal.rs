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

    pub fn secret_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    pub fn key(&self) -> Key {
        hex::encode(self.0.verifying_key().to_bytes())
    }

    fn sign(&self, bytes: &[u8]) -> String {
        hex::encode(self.0.sign(bytes).to_bytes())
    }
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    pub key: Key,
    pub counter: u64,
    pub sig: String,
}

fn call_bytes(room: &str, verb: &str, args: &JsonObject, counter: u64) -> Vec<u8> {
    canonical(&json!({ "room": room, "verb": verb, "arguments": Value::Object(args.clone()), "counter": counter })).into_bytes()
}

/// Seal a call for one room. The arguments are sealed as they are now:
/// change them afterwards and the seal no longer matches.
pub fn seal_call(keypair: &Keypair, room: &str, params: &mut CallToolRequestParams, counter: u64) {
    let args = params.arguments.clone().unwrap_or_default();
    let seal = Seal { key: keypair.key(), counter, sig: keypair.sign(&call_bytes(room, &params.name, &args, counter)) };
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

/// Re-check a seal kept in a record, against the verb and arguments kept with it.
pub fn recheck(room: &str, verb: &str, args: &JsonObject, seal: &Seal) -> bool {
    verify(&seal.key, &call_bytes(room, verb, args, seal.counter), &seal.sig)
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
    fn statements_and_keys_round_trip() {
        let maya = Keypair::from_seed("maya");
        assert_eq!(maya.key(), Keypair::from_seed("maya").key(), "stand-in keys are stable");
        assert_eq!(Keypair::from_secret_hex(&maya.secret_hex()).unwrap().key(), maya.key());
        let t = tether(&maya, "abc", "site-fixes");
        assert!(t.holds());
        let mut forged = t.clone();
        forged.body = json!({ "agent": "xyz", "name": "site-fixes" });
        assert!(!forged.holds());
        assert_eq!(canonical(&json!({ "b": 1, "a": { "d": 2, "c": 3 } })), r#"{"a":{"c":3,"d":2},"b":1}"#);
    }
}

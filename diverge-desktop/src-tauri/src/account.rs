//! Your account, made when the first-run page is finished.
//!
//! [`make`] draws twelve BIP-39 recovery words, derives the account's root
//! from them (SLIP-10, in `diverge_desktop_room::account`), has the root
//! sign the genesis and a first device list naming this Mac's key, and
//! drops the root before it returns: nothing outside `make` ever holds it,
//! and what held it is wiped.
//!
//! The words are kept in your keys file, sealed (ChaCha20-Poly1305) under a
//! key drawn with HKDF from this Mac's device key and bound to the account,
//! and marked unconfirmed until you've written them down. The sealing key
//! lives in the same owner-only file, so the seal keeps the words out of
//! plain sight (a search of the folder, a file pasted somewhere); it adds
//! nothing against someone who already holds the file. Nothing in the app
//! opens them yet, and no door can read them.

use bip39::Mnemonic;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key as AeadKey, Nonce};
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

use diverge_desktop_room::Keypair;
use diverge_desktop_room::account::{self as proof, Proof};

/// Twelve words are 16 bytes of entropy.
pub const ENTROPY: usize = 16;

/// Your recovery words, sealed, and whether you've confirmed writing them down.
#[derive(Serialize, Deserialize, Clone)]
pub struct SealedWords {
    pub nonce: String,
    pub sealed: String,
    pub confirmed: bool,
}

/// A new account: its proof, and its words sealed.
pub struct Made {
    pub proof: Proof,
    pub words: SealedWords,
}

/// New entropy for new words.
pub fn entropy() -> Zeroizing<[u8; ENTROPY]> {
    let mut bytes = Zeroizing::new([0u8; ENTROPY]);
    getrandom::getrandom(bytes.as_mut()).expect("the system has randomness");
    bytes
}

/// The same entropy every run: for the invented people of tests and the
/// browser preview's snapshot only.
pub fn seeded_entropy(seed: &str) -> Zeroizing<[u8; ENTROPY]> {
    use sha2::Digest;
    let digest: [u8; 32] = Sha256::digest(format!("diverge-desktop stand-in words: {seed}").as_bytes()).into();
    let mut bytes = Zeroizing::new([0u8; ENTROPY]);
    bytes.copy_from_slice(&digest[..ENTROPY]);
    bytes
}

/// Make an account whose first device is `device`: the words, the root
/// they make, the genesis and the first device list. The root is dropped
/// here, and the seed it came from is wiped.
pub fn make(device: &Keypair, entropy: Zeroizing<[u8; ENTROPY]>, at: DateTime<Utc>) -> Made {
    let words = Mnemonic::from_entropy(entropy.as_ref()).expect("16 bytes make twelve words");
    let mut seed = words.to_seed("");
    let root = proof::root(&seed);
    seed.zeroize();
    let genesis = proof::genesis(&root, at);
    let id = proof::account_id(&genesis);
    let devices = proof::device_list(&root, &id, 1, &[device.key()]);
    drop(root);
    let phrase = Zeroizing::new(words.to_string());
    drop(words);
    Made { words: seal(device, &id, &phrase), proof: Proof { genesis, devices } }
}

/// The key the words are sealed under: from this Mac's device key, for one account.
fn sealing_key(device: &Keypair, account: &str) -> Zeroizing<[u8; 32]> {
    let secret = Zeroizing::new(hex::decode(Zeroizing::new(device.secret_hex()).as_str()).expect("a key's secret is hex"));
    let mut key = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(account.as_bytes()), &secret).expand(b"diverge-desktop recovery words", key.as_mut()).expect("32 bytes is a length HKDF gives");
    key
}

fn seal(device: &Keypair, account: &str, phrase: &str) -> SealedWords {
    let key = sealing_key(device, account);
    let mut nonce = [0u8; 12];
    getrandom::getrandom(&mut nonce).expect("the system has randomness");
    let cipher = ChaCha20Poly1305::new(AeadKey::from_slice(key.as_ref()));
    let sealed = cipher.encrypt(Nonce::from_slice(&nonce), Payload { msg: phrase.as_bytes(), aad: account.as_bytes() }).expect("sealing a short text");
    SealedWords { nonce: hex::encode(nonce), sealed: hex::encode(sealed), confirmed: false }
}

/// The words, opened: only tests, for now. Showing them to you comes with
/// confirming them; nothing else may read them.
#[cfg(test)]
pub fn open(device: &Keypair, account: &str, words: &SealedWords) -> Result<Zeroizing<String>, String> {
    let key = sealing_key(device, account);
    let cipher = ChaCha20Poly1305::new(AeadKey::from_slice(key.as_ref()));
    let nonce = hex::decode(&words.nonce).map_err(|e| e.to_string())?;
    if nonce.len() != 12 {
        return Err("a nonce is 12 bytes".into());
    }
    let sealed = hex::decode(&words.sealed).map_err(|e| e.to_string())?;
    let plain = cipher.decrypt(Nonce::from_slice(&nonce), Payload { msg: &sealed, aad: account.as_bytes() }).map_err(|_| "these words don't open with this key".to_string())?;
    String::from_utf8(plain).map(Zeroizing::new).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_make_the_root_that_signed_and_open_only_for_their_account() {
        let device = Keypair::from_seed("this mac");
        let made = make(&device, seeded_entropy("maya"), Utc::now());
        let account = made.proof.check().unwrap();
        assert_eq!((account.sequence, account.devices.clone()), (1, vec![device.key()]));
        let words = open(&device, &account.id, &made.words).unwrap();
        assert_eq!(words.split(' ').count(), 12, "twelve words");
        assert!(!made.words.confirmed, "not confirmed until you say you wrote them down");
        // The words make the root the genesis names: they are this account's way back.
        let root = proof::root(&Mnemonic::parse(words.as_str()).unwrap().to_seed(""));
        assert_eq!(root.key(), account.root);
        assert!(open(&device, "another account", &made.words).is_err());
        assert!(open(&Keypair::from_seed("another mac"), &account.id, &made.words).is_err());
        assert!(!made.words.sealed.contains(&hex::encode(words.as_bytes())), "sealed, not just written down another way");
    }
}

//! An identity's key: the private half of what its tokens name.

use std::io;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer as _, SigningKey};
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;
use zeroize::{Zeroize as _, Zeroizing};

use super::identities::folder_owned;
use super::name::Paths;
use super::{Error, Identities, Name};
use crate::file_lock;
use crate::wire::frame::auth::{KEY_LEN, SIGNATURE_LEN};
use crate::wire::handshake::Signer;

/// An identity's key on this host: an Ed25519 key pair, RFC 8032,
/// whose public half every ID token of the identity names — in its
/// `cnf` claim — and whose private half signs each handshake the
/// identity is presented in. The rules:
///
/// 1. **One per identity, per host.** The key is kept beside the
///    identity's file, as `<subject>.key`, and never leaves the host.
///    The same identity signed in on another host has another key
///    there: a token names the key of the host that asked for it.
/// 2. **Made when first needed.** A login makes the key where the
///    identity has none, and so does the first refresh of an identity
///    that was signed in before it had one.
///    [`key_generate`](Identities::key_generate) makes one on demand.
/// 3. **Never replaced by accident.** A key that exists is replaced
///    only by [`key_generate`](Identities::key_generate) told to
///    overwrite it. The old key is then gone, and so is every token
///    that named it: the next token asked for is refreshed, naming the
///    new key.
/// 4. **Sent only as its public half.** Every token request carries
///    the public key, as [`jwk`](Self::jwk), and the issuer writes it
///    into the ID token it answers. The private half is read by this
///    module alone, to sign.
/// 5. **Kept as the file is.** Written whole to a staging file, made
///    owner-only, and renamed over the key, under the identity's lock.
///    The file is a JSON Web Key, RFC 8037, private part included.
/// 6. **Gone with the identity.** Removing an identity removes its
///    key.
///
/// It has no `Serialize` and no `Display`, its `Debug` prints
/// `Key(<hidden>)`, and its memory is zeroed when it is dropped.
#[derive(Clone)]
pub struct Key(SigningKey);

/// The key's file: a JSON Web Key, RFC 8037, with its private part.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    kty: String,
    crv: String,
    /// The public key, as unpadded base64url.
    x: String,
    /// The private key, as unpadded base64url.
    d: String,
}

/// The private part is zeroed as the file's contents are let go.
impl Drop for Stored {
    fn drop(&mut self) {
        self.d.zeroize();
    }
}

/// The key type every key here is.
const KTY: &str = "OKP";

/// The curve every key here is on.
const CRV: &str = "Ed25519";

impl Key {
    /// A new key, from the system's randomness.
    fn generate() -> Key {
        let mut seed = Zeroizing::new([0u8; KEY_LEN]);
        rand::rng().fill_bytes(seed.as_mut());
        Key(SigningKey::from_bytes(&seed))
    }

    /// The public half: thirty-two bytes.
    pub fn public(&self) -> [u8; KEY_LEN] {
        self.0.verifying_key().to_bytes()
    }

    /// The public half as the JSON Web Key a token request carries and
    /// a token names: `{"kty":"OKP","crv":"Ed25519","x":"…"}`, those
    /// three members and no other.
    pub fn jwk(&self) -> String {
        format!(r#"{{"kty":"{KTY}","crv":"{CRV}","x":"{}"}}"#, URL_SAFE_NO_PAD.encode(self.public()))
    }
}

impl Signer for Key {
    fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN] {
        self.0.sign(message).to_bytes()
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(<hidden>)")
    }
}

/// The key at `paths`, or `None` when there is no such file. A file
/// that is not a key this module wrote is [`Error::Key`]. The caller
/// holds the lock.
pub(super) async fn read(paths: &Paths) -> Result<Option<Key>, Error> {
    let bytes = match tokio::fs::read(&paths.key).await {
        Ok(bytes) => Zeroizing::new(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Io { path: paths.key.clone(), source }),
    };
    let not_one = |why: &str| Error::Key { path: paths.key.clone(), why: why.to_string() };
    let stored: Stored = serde_json::from_slice(&bytes).map_err(|error| not_one(&error.to_string()))?;
    if stored.kty != KTY || stored.crv != CRV {
        return Err(not_one("it is not an Ed25519 key"));
    }
    let seed = Zeroizing::new(URL_SAFE_NO_PAD.decode(stored.d.as_bytes()).map_err(|_| not_one("its private part is not base64url"))?);
    let seed: &[u8; KEY_LEN] = seed.as_slice().try_into().map_err(|_| not_one("its private part is not thirty-two bytes"))?;
    let key = Key(SigningKey::from_bytes(seed));
    // The public part is the private part's own: a file whose two
    // halves disagree was not written here.
    if URL_SAFE_NO_PAD.encode(key.public()) != stored.x {
        return Err(not_one("its public part is not its private part's"));
    }
    Ok(Some(key))
}

/// Write `key` at `paths`, as rule 5 says. The caller holds the lock,
/// and the folder exists.
pub(super) async fn write(paths: &Paths, key: &Key) -> Result<(), Error> {
    let staging = &paths.key_staging;
    let failed = |source| Error::Io { path: staging.clone(), source };
    let stored = Stored {
        kty: KTY.to_string(),
        crv: CRV.to_string(),
        x: URL_SAFE_NO_PAD.encode(key.public()),
        d: URL_SAFE_NO_PAD.encode(key.0.to_bytes()),
    };
    let bytes = Zeroizing::new(serde_json::to_vec_pretty(&stored).map_err(|source| Error::Json { path: staging.clone(), source })?);
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(staging).await.map_err(failed)?;
    file.write_all(&bytes).await.map_err(failed)?;
    file.sync_all().await.map_err(failed)?;
    drop(file);
    tokio::fs::rename(staging, &paths.key).await.map_err(|source| Error::Io { path: paths.key.clone(), source })
}

/// The key at `paths`, made and written where there is none — rule 2.
/// The caller holds the lock, and the folder exists.
pub(super) async fn ensure(paths: &Paths) -> Result<Key, Error> {
    if let Some(key) = read(paths).await? {
        return Ok(key);
    }
    let key = Key::generate();
    write(paths, &key).await?;
    Ok(key)
}

impl Identities {
    /// Make a key for `name`, under its lock, and answer its public
    /// half as a JSON Web Key — see [`Key`], rules 2 and 3. Where the
    /// identity has a key already: replaced when `overwrite` is set,
    /// else [`Error::KeyExists`], and nothing changed. The identity
    /// need not be signed in: a key made first is the one its login
    /// uses.
    pub async fn key_generate(&self, name: &Name, overwrite: bool) -> Result<String, Error> {
        let paths = self.paths(name);
        folder_owned(&paths.folder).await?;
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        if !overwrite && read(&paths).await?.is_some() {
            return Err(Error::KeyExists(name.clone()));
        }
        let key = Key::generate();
        write(&paths, &key).await?;
        drop(held);
        Ok(key.jwk())
    }
}

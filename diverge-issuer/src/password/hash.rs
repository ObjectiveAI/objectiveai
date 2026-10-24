//! Hashing a password.

use argon2::Argon2;
use argon2::password_hash::PasswordHasher;

use super::Error;

/// The PHC string of `password`'s argon2id hash, under a fresh random
/// salt.
pub async fn hash(password: &str) -> Result<String, Error> {
    let password = password.to_string();
    let hashed = tokio::task::spawn_blocking(move || Argon2::default().hash_password(password.as_bytes()).map(|hash| hash.to_string()));
    hashed.await.map_err(|_| Error::Panicked)?.map_err(Error::Hash)
}

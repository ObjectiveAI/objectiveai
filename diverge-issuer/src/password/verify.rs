//! Checking a password.

use argon2::Argon2;
use argon2::password_hash::PasswordVerifier;
use tokio::sync::OnceCell;

use super::hash;

/// Whether `password` is the one `hash` is the hash of. With no hash —
/// an identity that does not exist — a password is checked against a
/// hash of nothing anyone knows, and refused, so a sign-in as an
/// identity that does not exist takes as long as one with a wrong
/// password and says no more.
pub async fn verify(password: &str, hash: Option<String>) -> bool {
    let (hash, known) = match hash {
        Some(hash) => (hash, true),
        None => match absent().await {
            Some(hash) => (hash.clone(), false),
            None => return false,
        },
    };
    let password = password.to_string();
    let verified = tokio::task::spawn_blocking(move || Argon2::default().verify_password(password.as_bytes(), hash.as_str()).is_ok());
    known && verified.await.unwrap_or(false)
}

/// The hash a sign-in as an identity that does not exist is checked
/// against: made once, of a password minted then and never kept.
async fn absent() -> Option<&'static String> {
    static ABSENT: OnceCell<Option<String>> = OnceCell::const_new();
    ABSENT
        .get_or_init(|| async {
            hash(&crate::tokens::mint()).await.ok()
        })
        .await
        .as_ref()
}

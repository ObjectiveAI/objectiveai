//! Naming an account.

/// Which account a request means: its identity, `{"issuer": …,
/// "subject": …}`. One account per identity, so each names exactly
/// one.
pub type Reference = crate::shared::Identity;

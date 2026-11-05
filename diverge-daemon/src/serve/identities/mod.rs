//! The host's identities, listed — the issuers they are signed in at,
//! and the subjects at one issuer, each one answer read from the files
//! as they are then — and one signed in, by the device authorization
//! grant, on this host.

pub mod issuers;
pub mod login;
pub mod subjects;

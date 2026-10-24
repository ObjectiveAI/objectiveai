//! A name, made safe for a path.

use super::encode::encode;

/// `name` as one folder's or one file's name on every platform: the
/// encoding an identity's own folder and file are named by — every
/// byte but `A–Z a–z 0–9 - . _ ~` as `%XX`, and a name Windows
/// reserves, or a trailing `.`, escaped too. Distinct names give
/// distinct results, so a program that keeps something per identity
/// may name it by its issuer's and its subject's `path_name`.
pub fn path_name(name: &str) -> String {
    encode(name)
}

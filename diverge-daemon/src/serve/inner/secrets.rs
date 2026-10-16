//! A record's secrets, checked against its template and the grants.

use diverge_sdk::daemon::grant::vault::Over;
use indexmap::IndexMap;
use sqlx::PgConnection;

use crate::judge::{self, Standing};
use crate::store::{self, vault};

/// What checking a record's secrets came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Secrets {
    /// Every name the template declares is bound, nothing else is,
    /// and every key bound is reached.
    Ok,
    /// The names are not the template's: what is missing, or what is
    /// not declared, in a sentence.
    Mismatch(String),
    /// A key bound is one the grants do not reach.
    Forbidden,
}

/// Check `bound`, a record's secrets, against `declared`, its
/// template's: every declared name bound and no other, else
/// `Mismatch`; then every key bound reached by a `use` grant — a key
/// the vault holds by the grants' `within`, a key it does not hold
/// yet only by a grant whose `within` is `any` — else `Forbidden`.
/// Nothing is read of the keys' values.
pub async fn secrets(conn: &mut PgConnection, standing: &Standing, declared: &[String], bound: &IndexMap<String, String>) -> Result<Secrets, store::Error> {
    let missing: Vec<&str> = declared.iter().filter(|name| !bound.contains_key(name.as_str())).map(|name| name.as_str()).collect();
    if !missing.is_empty() {
        return Ok(Secrets::Mismatch(format!("the template declares secrets the record does not bind: {}", missing.join(", "))));
    }
    let extra: Vec<&str> = bound.keys().filter(|name| !declared.contains(*name)).map(|name| name.as_str()).collect();
    if !extra.is_empty() {
        return Ok(Secrets::Mismatch(format!("the record binds secrets the template does not declare: {}", extra.join(", "))));
    }
    if bound.is_empty() {
        return Ok(Secrets::Ok);
    }
    if !judge::vault::holds(standing, Over::Use) {
        return Ok(Secrets::Forbidden);
    }
    for key in bound.values() {
        let reached = match vault::by_key(conn, key, false).await? {
            Some(entry) => judge::vault::over(standing, Over::Use, &entry),
            None => judge::vault::any(standing, Over::Use),
        };
        if !reached {
            return Ok(Secrets::Forbidden);
        }
    }
    Ok(Secrets::Ok)
}

/// What a template's declaration must be: no name empty, no two
/// alike; else why not, in a sentence.
pub fn declared(names: &[String]) -> Result<(), String> {
    for (index, name) in names.iter().enumerate() {
        if name.is_empty() {
            return Err("a secret's name is empty".to_string());
        }
        if names[..index].contains(name) {
            return Err(format!("the secret {name} is declared twice"));
        }
    }
    Ok(())
}

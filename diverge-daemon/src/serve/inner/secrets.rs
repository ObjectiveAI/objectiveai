//! A record's secrets, checked against its template, the grants and
//! the vault.

use diverge_sdk::daemon::grant::vault::Over;
use indexmap::IndexMap;
use sqlx::PgConnection;

use crate::judge::{self, Standing};
use crate::store::{self, vault};

/// What checking a record's secrets came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Secrets {
    /// Every name the template declares is bound, nothing else is,
    /// and every key bound is reached, held by the vault, and has a
    /// value.
    Ok,
    /// The names are not the template's: what is missing, or what is
    /// not declared, in a sentence.
    Mismatch(String),
    /// A key bound is one the vault does not hold, or holds with no
    /// value: the secret and the key, in a sentence.
    Missing(String),
    /// A key bound is one the grants do not reach.
    Forbidden,
}

/// Check `bound`, a record's secrets, against `declared`, its
/// template's. The rules, in order:
///
/// 1. A declared name not bound, or a bound name not declared:
///    `Mismatch`, naming them.
/// 2. Nothing bound: `Ok`.
/// 3. No `use` grant over any key at all: `Forbidden`.
/// 4. For each key bound, in the binding's order: a key the vault
///    holds that the grants' `within` does not reach is `Forbidden`;
///    one it holds with no slot is `Missing`; one it does not hold is
///    `Missing` to a standing whose `use` reaches every key —
///    [`judge::vault::any`] — and `Forbidden` to any other, which is
///    told nothing of what the vault has.
/// 5. Else `Ok`.
///
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
    for (name, key) in bound {
        match vault::by_key(conn, key, false).await? {
            Some(entry) => {
                if !judge::vault::over(standing, Over::Use, &entry) {
                    return Ok(Secrets::Forbidden);
                }
                if entry.slots.is_empty() {
                    return Ok(Secrets::Missing(format!("the secret {name} is bound to the vault key {key}, which holds no value")));
                }
            }
            None => {
                if !judge::vault::any(standing, Over::Use) {
                    return Ok(Secrets::Forbidden);
                }
                return Ok(Secrets::Missing(format!("the secret {name} is bound to the vault key {key}, which the vault does not hold")));
            }
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

//! A run's secrets: the names a template declares, bound to vault
//! keys by a record, read from the vault at the start.

use indexmap::IndexMap;
use serde_json::Value;

use super::StartError;
use crate::daemon::Daemon;
use crate::store::vault;

/// The binding a template's names project to: each declared name, in
/// the template's order, to the vault key `binding` holds it under.
/// A name the binding does not hold is why not, in a sentence — for
/// a dependency, a name the agent that spawns it does not bind, since
/// its own template does not declare it.
pub fn bind(declared: &[String], binding: &IndexMap<String, String>) -> Result<IndexMap<String, String>, String> {
    let mut bound = IndexMap::with_capacity(declared.len());
    for name in declared {
        let Some(key) = binding.get(name) else {
            return Err(format!("the secret {name} is not one the agent binds"));
        };
        bound.insert(name.clone(), key.clone());
    }
    Ok(bound)
}

/// The values the binding's secrets are handed as: each bound vault
/// key read now, in the binding's order. A key the vault does not
/// hold is the start's failure, naming the secret and the key;
/// nothing of the run is made before every one is read.
pub async fn resolve(daemon: &Daemon, bound: &IndexMap<String, String>) -> Result<IndexMap<String, Value>, StartError> {
    if bound.is_empty() {
        return Ok(IndexMap::new());
    }
    let keys: Vec<String> = bound.values().cloned().collect();
    let values = {
        let mut conn = daemon.store.acquire().await?;
        vault::values(&mut conn, &keys).await?
    };
    let mut resolved = IndexMap::with_capacity(bound.len());
    for (name, key) in bound {
        let Some(value) = values.get(key) else {
            return Err(StartError::Secrets(format!("the secret {name} is bound to the vault key {key}, which is not set")));
        };
        resolved.insert(name.clone(), value.clone());
    }
    Ok(resolved)
}

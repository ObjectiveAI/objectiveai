//! A run's secrets: the names its program takes, each in its mode,
//! bound to vault keys by the record, and the slots held while the
//! run is active.
//!
//! A template declares names; a record binds each to a vault key; the
//! program's registration declares which names it takes and the MODE
//! of each — `fixed`, a value read and never held, or `rotating`, one
//! slot of the key held for the run's active period and written back
//! when it goes inactive. [`declare`] checks the program's names
//! against the template's, [`bind`] projects names onto the record's
//! binding, [`acquire`] takes the values at an activation, holding
//! one slot per rotating name, [`write_back`] puts what the program
//! made of them into the slots at an inactive, [`release`] frees the
//! slots unchanged at the run's end, and [`abandon`] gives back what
//! a registration took when no run came of it. No value is ever
//! formatted into an error: a sentence names secrets and keys alone.

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::secrets::{Declaration, Mode, Values};
use indexmap::IndexMap;

use super::StartError;
use super::answerers::Answerer;
use crate::daemon::{Daemon, Kind};
use crate::store::{self, vault};

/// What one run's secrets come to, from its registration on.
#[derive(Debug, Clone, PartialEq)]
pub struct Secrets {
    /// The names the program takes, each with its mode, as its
    /// registration answered: fixed for the run's life.
    pub declared: Declaration,
    /// Each declared name to the vault key the record binds it to, in
    /// the declaration's order — a dependency's, through the agent
    /// that spawns it.
    pub bound: IndexMap<String, String>,
    /// The slots held now: each rotating name to the slot of its key
    /// the run holds, while the run is active — empty for a
    /// declaration with no rotating name, held still — and `None`
    /// while it is inactive, nothing held.
    pub held: Option<IndexMap<String, u32>>,
}

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

/// Whether the program's declaration takes only names the template
/// declares: the first that is not, in a sentence. A program may
/// take fewer than the template declares, never more.
pub fn declare(template: &[String], declaration: &Declaration) -> Result<(), String> {
    for name in declaration.keys() {
        if !template.contains(name) {
            return Err(format!("the program takes the secret {name}, which the template does not declare"));
        }
    }
    Ok(())
}

/// The values the run is handed at an activation, in the
/// declaration's order, every one taken in one transaction, so that
/// a failure holds nothing. For each name, in order: a `fixed` name's
/// value is a random slot of its key, read and never held; a
/// `rotating` name's is a random FREE slot of its key, held for
/// `holder` — the record — from the commit on. A key with no slot,
/// or none free for a rotating name, is the failure, naming the
/// secret and the key, and the transaction is let go with nothing
/// held. On success the slots taken are the run's `held`.
pub async fn acquire(daemon: &Daemon, secrets: &mut Secrets, holder: &Creator) -> Result<Values, StartError> {
    let mut tx = daemon.store.begin().await?;
    let mut values = Values::with_capacity(secrets.declared.len());
    let mut held = IndexMap::new();
    for (name, mode) in &secrets.declared {
        let Some(key) = secrets.bound.get(name) else {
            return Err(StartError::Secrets(format!("the secret {name} is not bound")));
        };
        let value = match mode {
            Mode::Fixed => match vault::read_random(&mut tx, key).await? {
                Some(value) => value,
                None => return Err(StartError::Secrets(format!("the secret {name} is bound to the vault key {key}, which holds no value"))),
            },
            Mode::Rotating => match vault::acquire(&mut tx, key, holder).await? {
                Some((slot, value)) => {
                    held.insert(name.clone(), slot);
                    value
                }
                None => return Err(StartError::Secrets(format!("the secret {name} is bound to the vault key {key}, no value of which is free"))),
            },
        };
        values.insert(name.clone(), value);
    }
    tx.commit().await.map_err(store::Error::from)?;
    let taken = !held.is_empty();
    secrets.held = Some(held);
    if taken {
        daemon.live.changed(Kind::Vault);
    }
    Ok(values)
}

/// What the program made of its rotating secrets, written back at an
/// inactive: each held name's value in `values` put into the slot the
/// run holds for it and the slot freed, in one transaction; a held
/// name absent from `values` has its slot freed unchanged; a name in
/// `values` the run holds no slot for — a fixed one, or one it never
/// took — is ignored. Nothing held is nothing to do. The run holds
/// nothing after, whatever the store said: a store that refused
/// leaves the slots held in the vault under the record, which boot
/// frees, and the values with nowhere to go.
pub async fn write_back(daemon: &Daemon, secrets: &mut Secrets, holder: &Creator, values: &Values) -> Result<(), store::Error> {
    let Some(held) = secrets.held.take() else {
        return Ok(());
    };
    if held.is_empty() {
        return Ok(());
    }
    let outcome = settle(daemon, &secrets.bound, &held, holder, values).await;
    daemon.live.changed(Kind::Vault);
    outcome
}

/// The slots the run holds freed, their values as they are: what the
/// run's end does, and what an activation whose feed failed does.
/// Nothing held is nothing to do. The run holds nothing after,
/// whatever the store said, as for [`write_back`].
pub async fn release(daemon: &Daemon, secrets: &mut Secrets, holder: &Creator) -> Result<(), store::Error> {
    let Some(held) = secrets.held.take() else {
        return Ok(());
    };
    if held.is_empty() {
        return Ok(());
    }
    let outcome = settle(daemon, &secrets.bound, &held, holder, &Values::new()).await;
    daemon.live.changed(Kind::Vault);
    outcome
}

/// What a registration took, given back: the provider refused the
/// run after its registration was answered, so no run ever took the
/// state off the answerer. Nothing stashed is nothing to do; a store
/// that refuses leaves the slots for boot to free.
pub async fn abandon(daemon: &Daemon, answerer: &Answerer) {
    if let Some(mut secrets) = answerer.secrets.lock().await.take() {
        let _ = release(daemon, &mut secrets, &answerer.sender).await;
    }
}

/// Every held slot freed in one transaction — with the value `values`
/// holds for its name written in first, where it holds one.
async fn settle(daemon: &Daemon, bound: &IndexMap<String, String>, held: &IndexMap<String, u32>, holder: &Creator, values: &Values) -> Result<(), store::Error> {
    let mut tx = daemon.store.begin().await?;
    for (name, slot) in held {
        let Some(key) = bound.get(name) else {
            continue;
        };
        match values.get(name) {
            Some(value) => vault::write_back(&mut tx, key, *slot, holder, value).await?,
            None => vault::release(&mut tx, key, *slot, holder).await?,
        };
    }
    tx.commit().await?;
    Ok(())
}

//! Who you are in rooms: ours, not the wire's.
//!
//! Ronald's protocol has no person: identities are opaque per provider,
//! a room's program can't tell its members apart, and nothing on the wire
//! names a person. So the app keeps keys:
//!
//! - **Personas.** Your usual one, under your usual name, and any fresh ones
//!   you make at a door. A fresh persona is a different key, which nothing
//!   ties to your others unless you say so.
//! - **Agents.** Each agent gets its own key, tethered to the persona it acts
//!   for. The agent never holds it: the app seals what the agent does
//!   through the door.
//! - **Counters.** Every sealed call carries a counter that only goes up, so
//!   a call can't be sent twice.
//!
//! Kept in one file only this app reads (owner-only permissions), never the
//! system keychain: a keychain can ask for permission in a popup, and this
//! app has none.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rmcp::model::CallToolRequestParams;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use diverge_desktop_room::{Key, Keypair, Statement, seal_call, tether};

#[derive(Serialize, Deserialize, Clone)]
struct PersonaRecord {
    id: String,
    name: String,
    secret: String,
    created: DateTime<Utc>,
    usual: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct AgentRecord {
    secret: String,
    persona: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Keys {
    personas: Vec<PersonaRecord>,
    agents: BTreeMap<String, AgentRecord>,
    counters: BTreeMap<Key, u64>,
    /// Which persona you are in each room you're in.
    rooms: BTreeMap<String, String>,
}

/// Who is acting: one of your personas, or one of your agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    Persona(String),
    Agent(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub key: Key,
    pub usual: bool,
    pub created: DateTime<Utc>,
}

pub struct Identity {
    file: Option<PathBuf>,
    keys: Mutex<Keys>,
}

fn persona_view(p: &PersonaRecord) -> Persona {
    let key = Keypair::from_secret_hex(&p.secret).map(|k| k.key()).unwrap_or_default();
    Persona { id: p.id.clone(), name: p.name.clone(), key, usual: p.usual, created: p.created }
}

impl Identity {
    /// Keys kept in `file`, made on first use under `usual_name`.
    pub fn open(file: PathBuf, usual_name: &str) -> Self {
        let keys: Keys = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let identity = Identity { file: Some(file), keys: Mutex::new(keys) };
        identity.ensure_usual(usual_name, None);
        identity
    }

    /// Keys that live only in memory, made from fixed seeds: tests and the
    /// browser preview's snapshot, so they come out the same every run.
    #[allow(dead_code)] // tests and the browser preview's snapshot
    pub fn stand_in(usual_name: &str) -> Self {
        let identity = Identity { file: None, keys: Mutex::new(Keys::default()) };
        identity.ensure_usual(usual_name, Some(Keypair::from_seed(usual_name)));
        identity
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Keys> {
        self.keys.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn save(&self, keys: &Keys) {
        let Some(file) = &self.file else { return };
        if let Ok(json) = serde_json::to_string_pretty(keys) {
            let _ = std::fs::write(file, json);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
            }
        }
    }

    fn ensure_usual(&self, name: &str, keypair: Option<Keypair>) {
        let mut keys = self.lock();
        if keys.personas.iter().any(|p| p.usual) {
            return;
        }
        let keypair = keypair.unwrap_or_else(Keypair::generate);
        keys.personas.push(PersonaRecord { id: "usual".into(), name: name.into(), secret: keypair.secret_hex(), created: Utc::now(), usual: true });
        self.save(&keys);
    }

    pub fn usual(&self) -> Persona {
        let keys = self.lock();
        keys.personas.iter().find(|p| p.usual).map(persona_view).expect("there is always a usual persona")
    }

    pub fn personas(&self) -> Vec<Persona> {
        self.lock().personas.iter().map(persona_view).collect()
    }

    pub fn persona(&self, id: &str) -> Option<Persona> {
        self.lock().personas.iter().find(|p| p.id == id).map(persona_view)
    }

    pub fn persona_by_key(&self, key: &str) -> Option<Persona> {
        self.personas().into_iter().find(|p| p.key == key)
    }

    /// A fresh persona: a new key, under a name you choose for one room.
    pub fn fresh(&self, name: &str) -> Result<Persona, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("a persona needs a name".into());
        }
        let keypair = Keypair::generate();
        let mut keys = self.lock();
        let id = format!("persona-{}", keys.personas.len() + 1);
        let record = PersonaRecord { id, name: name.into(), secret: keypair.secret_hex(), created: Utc::now(), usual: false };
        let view = persona_view(&record);
        keys.personas.push(record);
        self.save(&keys);
        Ok(view)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("a persona needs a name".into());
        }
        let mut keys = self.lock();
        let p = keys.personas.iter_mut().find(|p| p.id == id).ok_or("no such persona")?;
        p.name = name.into();
        self.save(&keys);
        Ok(())
    }

    /// The persona you are in a room, if you're in it.
    pub fn in_room(&self, room: &str) -> Option<Persona> {
        let id = self.lock().rooms.get(room).cloned()?;
        self.persona(&id)
    }

    pub fn set_room(&self, room: &str, persona: &str) {
        let mut keys = self.lock();
        keys.rooms.insert(room.into(), persona.into());
        self.save(&keys);
    }

    pub fn forget_room(&self, room: &str) {
        let mut keys = self.lock();
        keys.rooms.remove(room);
        self.save(&keys);
    }

    fn persona_keypair(&self, id: &str) -> Option<Keypair> {
        self.lock().personas.iter().find(|p| p.id == id).and_then(|p| Keypair::from_secret_hex(&p.secret).ok())
    }

    /// An agent's key, made on first use and tethered to the usual persona.
    /// In a room where you're someone else, `agent_in` tethers a key per persona.
    fn agent_keypair(&self, agent: &str, persona: &str) -> Keypair {
        let slot = if persona == "usual" { agent.to_owned() } else { format!("{agent}@{persona}") };
        let mut keys = self.lock();
        if let Some(k) = keys.agents.get(&slot).and_then(|a| Keypair::from_secret_hex(&a.secret).ok()) {
            return k;
        }
        let keypair = if self.file.is_none() { Keypair::from_seed(&format!("agent {slot}")) } else { Keypair::generate() };
        keys.agents.insert(slot, AgentRecord { secret: keypair.secret_hex(), persona: persona.into() });
        self.save(&keys);
        keypair
    }

    /// The key an agent acts under in a room, and its tether to the persona
    /// you are there. In a room where you're anonymous, so is your agent.
    pub fn agent_in(&self, agent: &str, room: Option<&str>) -> (Key, Statement) {
        let persona = room.and_then(|r| self.lock().rooms.get(r).cloned()).unwrap_or_else(|| "usual".into());
        let keypair = self.agent_keypair(agent, &persona);
        let person = self.persona_keypair(&persona).unwrap_or_else(|| self.persona_keypair("usual").expect("usual persona"));
        (keypair.key(), tether(&person, &keypair.key(), agent))
    }

    /// Whose key it is, if it's one of yours: a persona's name or an agent's.
    pub fn owner_of(&self, key: &str) -> Option<String> {
        if let Some(p) = self.persona_by_key(key) {
            return Some(p.name);
        }
        let keys = self.lock();
        keys.agents.iter().find(|(_, a)| Keypair::from_secret_hex(&a.secret).map(|k| k.key() == key).unwrap_or(false)).map(|(slot, _)| slot.split('@').next().unwrap_or(slot).to_owned())
    }

    fn keypair_for(&self, actor: &Actor, room: &str) -> Result<Keypair, String> {
        match actor {
            Actor::Persona(id) => self.persona_keypair(id).ok_or_else(|| "no such persona".into()),
            Actor::Agent(name) => {
                let persona = self.lock().rooms.get(room).cloned().unwrap_or_else(|| "usual".into());
                Ok(self.agent_keypair(name, &persona))
            }
        }
    }

    /// Who you are in a room, as the actor that seals your calls there.
    pub fn you_in(&self, room: &str) -> Actor {
        Actor::Persona(self.in_room(room).map(|p| p.id).unwrap_or_else(|| "usual".into()))
    }

    /// Seal a call to a room as `actor`, with the next counter for its key.
    pub fn seal(&self, actor: &Actor, room: &str, params: &mut CallToolRequestParams) -> Result<Key, String> {
        let keypair = self.keypair_for(actor, room)?;
        let key = keypair.key();
        let counter = {
            let mut keys = self.lock();
            let next = keys.counters.get(&key).copied().unwrap_or(0) + 1;
            keys.counters.insert(key.clone(), next);
            self.save(&keys);
            next
        };
        seal_call(&keypair, room, params, counter);
        Ok(key)
    }

    /// A statement as one of your personas: a receipt for a room you host,
    /// a vouch, a settlement.
    pub fn state(&self, persona_key: &str, kind: &str, body: Value) -> Result<Statement, String> {
        let id = self.persona_by_key(persona_key).map(|p| p.id).ok_or("that key isn't one of yours")?;
        let keypair = self.persona_keypair(&id).ok_or("no such persona")?;
        Ok(Statement::make(&keypair, kind, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn personas_agents_and_counters() {
        let file = std::env::temp_dir().join(format!("diverge-desktop-identity-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&file);
        let me = Identity::open(file.clone(), "maya");
        let usual = me.usual();
        assert_eq!(usual.name, "maya");
        let fresh = me.fresh("lamp person").unwrap();
        assert_ne!(fresh.key, usual.key, "a fresh persona is a different key");
        me.set_room("room-1", &fresh.id);
        assert_eq!(me.you_in("room-1"), Actor::Persona(fresh.id.clone()));
        let (agent_there, t) = me.agent_in("site-fixes", Some("room-1"));
        let (agent_usual, _) = me.agent_in("site-fixes", None);
        assert_ne!(agent_there, agent_usual, "anonymous there, anonymous agent there");
        assert!(t.holds() && t.key == fresh.key);
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        me.seal(&Actor::Persona(fresh.id.clone()), "room-1", &mut params).unwrap();
        let first = diverge_desktop_room::seal::seal_of(&params).unwrap().counter;
        me.seal(&Actor::Persona(fresh.id.clone()), "room-1", &mut params).unwrap();
        assert_eq!(diverge_desktop_room::seal::seal_of(&params).unwrap().counter, first + 1);
        // Everything survives a restart of the app.
        let again = Identity::open(file.clone(), "someone else");
        assert_eq!(again.usual().key, usual.key);
        assert_eq!(again.in_room("room-1").unwrap().key, fresh.key);
        assert_eq!(again.owner_of(&agent_there).as_deref(), Some("site-fixes"));
        let _ = std::fs::remove_file(&file);
    }
}

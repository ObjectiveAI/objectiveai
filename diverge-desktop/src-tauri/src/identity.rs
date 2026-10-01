//! Who you are in rooms: ours, not the wire's.
//!
//! Ronald's protocol has no person: identities are opaque per provider,
//! a room's program can't tell its members apart, and nothing on the wire
//! names a person. So the app keeps keys:
//!
//! - **Your account.** Made when you finish the first-run page, never
//!   before: until then there is no you here, no name and no key that acts
//!   for you, and nothing is signed or sent. The app never names you from
//!   this Mac; the name is the one you type. The account is your usual
//!   self: its root, made from your recovery words, names this Mac's key as
//!   one of its devices, and that key is your usual persona's (see
//!   [`crate::account`]).
//! - **Personas.** Your usual one, under the name you typed, and any fresh
//!   ones you make at a door. A fresh persona is a different key, which
//!   nothing ties to your others unless you say so; it is never in your
//!   account.
//! - **Agents.** Each agent gets its own key, tethered to the persona it acts
//!   for. The agent never holds it: the app seals what the agent does
//!   through the door.
//! - **Counters.** Every sealed call carries a counter that only goes up, so
//!   a call can't be sent twice. A counter never falls behind the clock, so
//!   losing the counters file can't lock you out of a room. Calls from one
//!   key to one room go out one at a time, so they arrive in order.
//!
//! Kept in files only this app reads, owner-only from the moment they're
//! made, never the system keychain: a keychain can ask for permission in a
//! popup, and this app has none. The keys file is written whole to a new
//! file and swapped in, with the last good one kept beside it (see
//! [`crate::store`]). If it can't be read and neither can its backup, the
//! app signs nothing and says so; it never makes new keys over it. A keys
//! file a newer version of the app wrote, or one the system won't read
//! this time, is left exactly as it is, and so is its backup, whatever the
//! backup holds: the app signs nothing until it can read the file again.
//!
//! A keys file from before accounts is upgraded in place, and a copy of it
//! as it was is kept beside it ([`BEFORE_ACCOUNTS`]). Its usual key becomes
//! this Mac's device key once you finish the first-run page, so every room
//! it's in still knows it; nothing in it is lost.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rmcp::model::CallToolRequestParams;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use diverge_desktop_room::account::Proof;
use diverge_desktop_room::{Key, Keypair, Statement, seal_call, tether};

use crate::account::SealedWords;
use crate::store::{self, Read, Why};

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

/// Your account, as this Mac keeps it.
#[derive(Serialize, Deserialize, Clone)]
struct AccountRecord {
    /// The genesis and the newest device list the root signed.
    proof: Proof,
    /// Your recovery words, sealed.
    words: SealedWords,
    /// When you confirmed, on the first-run page, that you're 18 or older.
    adult_confirmed: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default)]
struct Keys {
    personas: Vec<PersonaRecord>,
    agents: BTreeMap<String, AgentRecord>,
    /// Counters lived here once; read to carry them over, never written here again.
    #[serde(default, skip_serializing)]
    counters: BTreeMap<Key, u64>,
    /// Which persona you are in each room you're in.
    rooms: BTreeMap<String, String>,
    /// None until the first-run page is finished.
    #[serde(default)]
    account: Option<AccountRecord>,
}

/// Who is acting: one of your personas, or one of your agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    Persona(String),
    Agent(String),
}

/// One of your agents as it appears in one room.
#[derive(Debug, Clone)]
pub struct AgentIn {
    pub key: Key,
    pub tether: Statement,
    /// The name it goes by there.
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub key: Key,
    pub usual: bool,
    pub created: DateTime<Utc>,
}

/// Where the first-run page stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstRun {
    /// Finished: you have an account, under the name you typed.
    Done,
    /// Nothing yet: no name, and no key that acts for you.
    New,
    /// A folder from before accounts: it has you under `name`, which an
    /// earlier version of the app took from this Mac's login.
    Earlier { name: String },
    /// The page can't be finished in this copy of the app: the words say why.
    Blocked(&'static str),
}

pub struct Identity {
    /// Where your keys are saved; `None`: nowhere.
    file: Option<PathBuf>,
    /// Keys made from fixed seeds: tests and the browser preview's snapshot.
    seeded: bool,
    keys: Mutex<Keys>,
    counters: Mutex<BTreeMap<Key, u64>>,
    /// Why nothing is signed, when nothing is.
    refused: Option<Refused>,
    /// One call at a time from one key to one room.
    turns: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

/// Why nothing is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// Your keys file can't be used, and is left exactly as it is: it and
    /// its backup won't parse, a newer version of the app wrote it, or the
    /// system won't read it this time.
    Keys { file: PathBuf, newer: bool },
    /// This copy of the app doesn't hold its folder; the words say why.
    Folder(&'static str),
}

/// What a call gets when the keys file couldn't be read.
/// The same words are the screen's, in `src/strings.ts`.
pub const UNREADABLE: &str = "Your keys file can't be read, so nothing is sent as you.";
/// What a call gets when a newer version of the app wrote the keys file.
/// The same words are the screen's, in `src/strings.ts`.
pub const NEWER: &str = "Your keys file was written by a newer version of this app, so nothing is sent as you.";
/// What a call gets before the first-run page is finished.
/// The same words are the screen's, in `src/strings.ts`.
pub const NOT_NAMED: &str = "Nothing is sent as you until you've said what people should call you.";
/// What finishing the first-run page gets without the 18-or-older line confirmed.
/// The same words are the screen's, in `src/strings.ts`.
pub const NOT_ADULT: &str = "Diverge is for adults. Confirm that you're 18 or older to go on.";
/// What finishing the first-run page gets with no name.
/// The same words are the screen's, in `src/strings.ts`.
pub const NO_NAME: &str = "Type the name people should call you.";

/// Where a keys file from before accounts is kept, as it was, once it's upgraded.
pub const BEFORE_ACCOUNTS: &str = "identity.v1.json";

impl Refused {
    pub fn words(&self) -> &'static str {
        match self {
            Refused::Keys { newer: false, .. } => UNREADABLE,
            Refused::Keys { newer: true, .. } => NEWER,
            Refused::Folder(words) => words,
        }
    }
}

fn counters_of(file: &Path) -> PathBuf {
    file.with_file_name("counters.json")
}

fn persona_view(p: &PersonaRecord) -> Persona {
    let key = Keypair::from_secret_hex(&p.secret).map(|k| k.key()).unwrap_or_default();
    Persona { id: p.id.clone(), name: p.name.clone(), key, usual: p.usual, created: p.created }
}

impl Identity {
    /// Keys kept in `file`. A damaged file falls back to its backup, and is
    /// set aside. If both are unusable, or a newer version of the app wrote
    /// the file, or the system won't read it, the app runs signing nothing,
    /// and neither file is touched. A folder with no keys yet stays that
    /// way until the first-run page is finished: nothing is made or written.
    pub fn open(file: PathBuf) -> Self {
        let backup = || store::read::<Keys>(&store::previous_of(&file), store::KEYS);
        let refuse = |newer| (Keys::default(), Some(Refused::Keys { file: file.clone(), newer }), None, None);
        // The keys, why nothing is signed, the file they came from (if not
        // the keys file itself), and why the keys file is set aside.
        let (keys, refused, from_backup, set_aside) = match store::read::<Keys>(&file, store::KEYS) {
            Read::Good(k) => (k, None, None, None),
            // Whatever the backup holds: going back to it would leave the newer keys behind.
            Read::Unusable(Why::Newer(_)) => refuse(true),
            // Nothing is known about what it holds, so nothing may replace it.
            Read::Failed(_) => refuse(false),
            Read::Missing => match backup() {
                Read::Missing => (Keys::default(), None, None, None),
                Read::Good(k) => (k, None, Some(store::previous_of(&file)), None),
                Read::Unusable(Why::Newer(_)) => refuse(true),
                Read::Unusable(_) | Read::Failed(_) => refuse(false),
            },
            Read::Unusable(why) => match backup() {
                // The last good one holds: carry on from it, and keep the damaged one aside.
                Read::Good(k) => (k, None, Some(store::previous_of(&file)), Some(why)),
                Read::Unusable(Why::Newer(_)) => refuse(true),
                Read::Missing | Read::Unusable(_) | Read::Failed(_) => refuse(false),
            },
        };
        let read_from = from_backup.clone().unwrap_or_else(|| file.clone());
        // From before accounts: keep it as it was, then write it anew.
        let upgrade = refused.is_none() && keys.account.is_none() && !keys.personas.is_empty() && store::version_on_disk(&read_from).is_some_and(|v| v < store::KEYS.version);
        if upgrade {
            let _ = store::keep_copy(&read_from, &file.with_file_name(BEFORE_ACCOUNTS));
        }
        if let Some(why) = set_aside {
            let _ = store::set_aside_noting(&file, &file, store::KEYS, why, store::CarriedOn::LastGood);
        }
        let mut counters = keys.counters.clone();
        if refused.is_none() {
            counters.extend(store::load::<BTreeMap<Key, u64>>(&counters_of(&file), store::COUNTERS).unwrap_or_default());
        }
        let identity = Identity {
            file: if refused.is_some() { None } else { Some(file) },
            seeded: false,
            keys: Mutex::new(keys),
            counters: Mutex::new(counters),
            refused,
            turns: Mutex::new(HashMap::new()),
        };
        if from_backup.is_some() || upgrade {
            identity.save(&identity.lock());
        }
        identity
    }

    /// Your keys as `file` (or its backup) holds them, touching nothing:
    /// for a copy of the app that doesn't hold its folder. It saves
    /// nothing and signs nothing, and `says` why.
    pub fn untouched(file: &Path, says: &'static str) -> Self {
        let keys = store::peek::<Keys>(file, store::KEYS).unwrap_or_default();
        Identity {
            file: None,
            seeded: false,
            keys: Mutex::new(keys),
            counters: Mutex::new(BTreeMap::new()),
            refused: Some(Refused::Folder(says)),
            turns: Mutex::new(HashMap::new()),
        }
    }

    /// Keys that live only in memory, made from fixed seeds, with the
    /// first-run page finished under `usual_name`: tests and the browser
    /// preview's snapshot, so they come out the same every run.
    #[allow(dead_code)] // tests and the browser preview's snapshot
    pub fn stand_in(usual_name: &str) -> Self {
        let identity = Identity { file: None, seeded: true, keys: Mutex::new(Keys::default()), counters: Mutex::new(BTreeMap::new()), refused: None, turns: Mutex::new(HashMap::new()) };
        identity.finish_first_run(usual_name, true).expect("an invented person finishes the first-run page");
        identity
    }

    /// Your keys file, when it can't be used: where it is, and whether a
    /// newer version of the app wrote it.
    pub fn broken(&self) -> Option<(&Path, bool)> {
        match &self.refused {
            Some(Refused::Keys { file, newer }) => Some((file, *newer)),
            _ => None,
        }
    }

    /// Where the first-run page stands.
    pub fn first_run(&self) -> FirstRun {
        let keys = self.lock();
        if keys.account.is_some() {
            return FirstRun::Done;
        }
        if let Some(refused) = &self.refused {
            return FirstRun::Blocked(refused.words());
        }
        match keys.personas.iter().find(|p| p.usual) {
            Some(p) => FirstRun::Earlier { name: p.name.clone() },
            None => FirstRun::New,
        }
    }

    /// Finish the first-run page: you're called `name`, and you've said
    /// you're 18 or older. Makes your account, whose first device is this
    /// Mac's key: your usual persona's from before accounts, if this folder
    /// has one, a new key otherwise. Until this, nothing is signed.
    pub fn finish_first_run(&self, name: &str, adult: bool) -> Result<Persona, String> {
        if let Some(refused) = &self.refused {
            return Err(refused.words().into());
        }
        let name = name.trim();
        if name.is_empty() {
            return Err(NO_NAME.into());
        }
        if !adult {
            return Err(NOT_ADULT.into());
        }
        let mut keys = self.lock();
        if keys.account.is_some() {
            return Err("the first-run page is already finished".into());
        }
        let now = Utc::now();
        if !keys.personas.iter().any(|p| p.usual) {
            let keypair = if self.seeded { Keypair::from_seed(name) } else { Keypair::generate() };
            keys.personas.push(PersonaRecord { id: "usual".into(), name: name.into(), secret: keypair.secret_hex(), created: now, usual: true });
        }
        let usual = keys.personas.iter_mut().find(|p| p.usual).expect("there is a usual persona now");
        usual.name = name.into();
        let device = Keypair::from_secret_hex(&usual.secret)?;
        let entropy = if self.seeded { crate::account::seeded_entropy(name) } else { crate::account::entropy() };
        let made = crate::account::make(&device, entropy, now);
        keys.account = Some(AccountRecord { proof: made.proof, words: made.words, adult_confirmed: now });
        let view = keys.personas.iter().find(|p| p.usual).map(persona_view).expect("there is a usual persona now");
        self.save(&keys);
        Ok(view)
    }

    /// Whether anything may be signed as you: not before the first-run
    /// page is finished, and not while the keys file can't be used or
    /// another copy holds the folder.
    pub fn ready(&self) -> Result<(), String> {
        if let Some(refused) = &self.refused {
            return Err(refused.words().into());
        }
        if self.lock().account.is_none() {
            return Err(NOT_NAMED.into());
        }
        Ok(())
    }

    /// Your account's id and its proof, once the first-run page is finished.
    #[cfg(test)]
    pub fn account(&self) -> Option<(String, Proof)> {
        self.lock().account.as_ref().map(|a| (a.proof.id(), a.proof.clone()))
    }

    /// A turn for one key in one room: hold it while sealing and sending, so
    /// that key's calls reach that room in the order their counters say.
    pub fn turn(&self, actor: &Actor, room: &str) -> Arc<tokio::sync::Mutex<()>> {
        let slot = format!("{actor:?}\u{0}{room}");
        self.turns.lock().unwrap_or_else(|p| p.into_inner()).entry(slot).or_default().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Keys> {
        self.keys.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn save(&self, keys: &Keys) {
        let Some(file) = &self.file else { return };
        let _ = store::save(file, store::KEYS, keys);
    }

    /// Your usual self: there is none until the first-run page is finished
    /// (or, in a folder from before accounts, the one it had).
    pub fn usual(&self) -> Result<Persona, String> {
        let keys = self.lock();
        keys.personas.iter().find(|p| p.usual).map(persona_view).ok_or_else(|| NOT_NAMED.to_owned())
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
        self.ready()?;
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
        self.ready()?;
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

    /// Who you are in a room: the persona you are there, or your usual self.
    pub fn who_in(&self, room: &str) -> Result<Persona, String> {
        self.in_room(room).map(Ok).unwrap_or_else(|| self.usual())
    }

    /// The persona you are in a room, if you're in it.
    pub fn in_room(&self, room: &str) -> Option<Persona> {
        let id = self.lock().rooms.get(room).cloned()?;
        self.persona(&id)
    }

    /// Remember which persona you are in a room: only once there's a you.
    pub fn set_room(&self, room: &str, persona: &str) {
        if self.ready().is_err() {
            return;
        }
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
        let keypair = if self.seeded { Keypair::from_seed(&format!("agent {slot}")) } else { Keypair::generate() };
        keys.agents.insert(slot, AgentRecord { secret: keypair.secret_hex(), persona: persona.into() });
        self.save(&keys);
        keypair
    }

    /// The key an agent acts under in a room, its tether to the persona you
    /// are there, and the name it goes by there. In a room where you're a
    /// fresh name, your agent gets a fresh key and a plain name ("lamp
    /// person's helper"), never the name it has with you. Nothing, before
    /// the first-run page is finished: a tether is signed.
    pub fn agent_in(&self, agent: &str, room: Option<&str>) -> Result<AgentIn, String> {
        self.ready()?;
        let persona = room.and_then(|r| self.lock().rooms.get(r).cloned()).unwrap_or_else(|| "usual".into());
        let keypair = self.agent_keypair(agent, &persona);
        let person = self.persona_keypair(&persona).or_else(|| self.persona_keypair("usual")).ok_or(NOT_NAMED)?;
        let name = if persona == "usual" {
            agent.to_owned()
        } else {
            let keys = self.lock();
            let who = keys.personas.iter().find(|p| p.id == persona).map(|p| p.name.clone()).unwrap_or_default();
            // The helpers of one fresh name, numbered by name.
            let n = keys.agents.iter().filter(|(_, a)| a.persona == persona).position(|(slot, _)| slot.split('@').next() == Some(agent)).unwrap_or(0);
            if n == 0 { format!("{who}'s helper") } else { format!("{who}'s helper {}", n + 1) }
        };
        Ok(AgentIn { key: keypair.key(), tether: tether(&person, &keypair.key(), &name), name })
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

    /// Seal a call to a room as `actor`, with the next counter for its key,
    /// saved before the call goes. Hold [`Identity::turn`] across sealing and
    /// sending so the room gets that key's calls in order.
    pub fn seal(&self, actor: &Actor, room: &str, params: &mut CallToolRequestParams) -> Result<Key, String> {
        self.ready()?;
        let keypair = self.keypair_for(actor, room)?;
        let key = keypair.key();
        let counter = {
            let mut counters = self.counters.lock().unwrap_or_else(|p| p.into_inner());
            // Never behind the clock: a lost counters file can't lock you out.
            let next = (counters.get(&key).copied().unwrap_or(0) + 1).max(Utc::now().timestamp_millis().max(0) as u64);
            counters.insert(key.clone(), next);
            if let Some(file) = &self.file {
                let _ = store::save(&counters_of(file), store::COUNTERS, &*counters);
            }
            next
        };
        seal_call(&keypair, room, params, counter);
        Ok(key)
    }

    /// A statement as one of your personas: a receipt for a room you host,
    /// a vouch, a settlement.
    pub fn state(&self, persona_key: &str, kind: &str, body: Value) -> Result<Statement, String> {
        self.ready()?;
        let id = self.persona_by_key(persona_key).map(|p| p.id).ok_or("that key isn't one of yours")?;
        let keypair = self.persona_keypair(&id).ok_or("no such persona")?;
        Ok(Statement::make(&keypair, kind, body))
    }

    /// Your recovery words, opened: tests only. Nothing else in the app
    /// reads them, and no door can.
    #[cfg(test)]
    pub(crate) fn words(&self) -> Option<zeroize::Zeroizing<String>> {
        let keys = self.lock();
        let account = keys.account.as_ref()?;
        let device = keys.personas.iter().find(|p| p.usual).and_then(|p| Keypair::from_secret_hex(&p.secret).ok())?;
        crate::account::open(&device, &account.proof.id(), &account.words).ok()
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
        let me = Identity::open(file.clone());
        let usual = me.finish_first_run("maya", true).unwrap();
        assert_eq!(usual.name, "maya");
        let fresh = me.fresh("lamp person").unwrap();
        assert_ne!(fresh.key, usual.key, "a fresh persona is a different key");
        me.set_room("room-1", &fresh.id);
        assert_eq!(me.you_in("room-1"), Actor::Persona(fresh.id.clone()));
        let there = me.agent_in("site-fixes", Some("room-1")).unwrap();
        let usual_agent = me.agent_in("site-fixes", None).unwrap();
        let agent_there = there.key.clone();
        assert_ne!(agent_there, usual_agent.key, "anonymous there, anonymous agent there");
        assert!(there.tether.holds() && there.tether.key == fresh.key);
        assert_eq!(there.name, "lamp person's helper", "and it isn't called site-fixes there");
        assert_eq!(there.tether.field("name"), Some("lamp person's helper"));
        assert_eq!(usual_agent.name, "site-fixes");
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        me.seal(&Actor::Persona(fresh.id.clone()), "room-1", &mut params).unwrap();
        let first = diverge_desktop_room::seal::seal_of(&params).unwrap().counter;
        me.seal(&Actor::Persona(fresh.id.clone()), "room-1", &mut params).unwrap();
        assert!(diverge_desktop_room::seal::seal_of(&params).unwrap().counter > first, "counters only go up");
        // Everything survives a restart of the app.
        let again = Identity::open(file.clone());
        assert_eq!(again.usual().unwrap().key, usual.key);
        assert_eq!(again.first_run(), FirstRun::Done);
        assert_eq!(again.in_room("room-1").unwrap().key, fresh.key);
        assert_eq!(again.owner_of(&agent_there).as_deref(), Some("site-fixes"));
        me.seal(&Actor::Persona(fresh.id.clone()), "room-1", &mut params).unwrap();
        assert!(diverge_desktop_room::seal::seal_of(&params).unwrap().counter > first + 1, "counters carry on after a restart");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600, "owner-only");
        }
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn a_damaged_keys_file_is_never_replaced_with_new_keys() {
        let dir = std::env::temp_dir().join(format!("diverge-desktop-keys-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or(0)));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("identity.json");
        let me = Identity::open(file.clone());
        let usual = me.finish_first_run("maya", true).unwrap();
        me.fresh("lamp person").unwrap();
        // Damaged, with a good backup: the backup carries on, the damaged one is kept aside.
        std::fs::write(&file, "{ not json").unwrap();
        let back = Identity::open(file.clone());
        assert!(back.broken().is_none());
        assert_eq!(back.usual().unwrap().key, usual.key, "the same keys, from the backup");
        assert!(std::fs::read_dir(&dir).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("damaged")));
        let aside = store::notices_under(&dir);
        assert_eq!((aside.len(), aside[0].kind, aside[0].carried_on), (1, "keys", store::CarriedOn::LastGood), "{aside:?}");
        // Damaged, and the backup too: nothing is signed, and neither file is touched.
        std::fs::write(&file, "{ not json").unwrap();
        std::fs::write(store::previous_of(&file), "also not json").unwrap();
        let broken = Identity::open(file.clone());
        assert_eq!(broken.broken(), Some((file.as_path(), false)));
        assert_eq!(broken.first_run(), FirstRun::Blocked(UNREADABLE), "and the first-run page can't make new keys over it");
        assert_eq!(broken.finish_first_run("maya", true).unwrap_err(), UNREADABLE);
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert_eq!(broken.seal(&Actor::Persona("usual".into()), "room-1", &mut params).unwrap_err(), UNREADABLE);
        assert!(broken.state(&usual.key, "vouch", json!({})).is_err());
        broken.fresh("anyone").ok();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{ not json", "left as it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keys_from_before_files_carried_a_version_carry_on() {
        let dir = store::tests::folder("keys-unversioned");
        let file = dir.join("identity.json");
        let keypair = Keypair::generate();
        let old = json!({ "personas": [{ "id": "usual", "name": "maya", "secret": keypair.secret_hex(), "created": Utc::now(), "usual": true }], "agents": {}, "rooms": {} });
        std::fs::write(&file, old.to_string()).unwrap();
        let me = Identity::open(file.clone());
        assert_eq!(me.usual().unwrap().key, keypair.key(), "the same keys");
        assert_eq!(store::header(&file).map(|h| (h.file, h.version)), Some(("keys".into(), store::KEYS.version)), "and now it says what it is");
        assert_eq!(std::fs::read_to_string(dir.join(BEFORE_ACCOUNTS)).unwrap(), old.to_string(), "kept as it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every file in a folder and what it holds.
    fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_file()).map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap())).collect()
    }

    fn assert_signs_nothing(me: &Identity, words: &str) {
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert_eq!(me.seal(&Actor::Persona("usual".into()), "room-1", &mut params).unwrap_err(), words);
        assert_eq!(me.state(&me.usual().map(|p| p.key).unwrap_or_default(), "vouch", json!({})).unwrap_err(), words);
        if words != NOT_NAMED {
            assert_eq!(me.finish_first_run("anyone", true).unwrap_err(), words, "and no account is made over keys it can't use");
        }
        me.fresh("anyone").ok();
        me.set_room("room-1", "usual");
        me.agent_in("site-fixes", Some("room-1")).ok();
    }

    #[test]
    fn a_keys_file_from_a_newer_version_is_left_alone() {
        let dir = store::tests::folder("keys-newer");
        let file = dir.join("identity.json");
        let newer = r#"{"file":"keys","version":99,"data":{"what":"this build can't read"}}"#;
        std::fs::write(&file, newer).unwrap();
        std::fs::write(store::previous_of(&file), newer).unwrap();
        let me = Identity::open(file.clone());
        assert_eq!(me.broken(), Some((file.as_path(), true)), "nothing is signed");
        assert_signs_nothing(&me, NEWER);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), newer, "never written over");
        assert_eq!(std::fs::read_to_string(store::previous_of(&file)).unwrap(), newer);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A newer version moves the old keys file to the backup when it first
    /// saves its own. This build must not go back to that backup: the
    /// newer keys would be left behind.
    #[test]
    fn a_newer_keys_file_with_an_older_backup_is_left_alone_and_the_backup_too() {
        let dir = store::tests::folder("keys-newer-older-backup");
        let file = dir.join("identity.json");
        let older = Identity::open(file.clone());
        older.finish_first_run("maya", true).unwrap();
        older.fresh("lamp person").unwrap();
        drop(older);
        std::fs::rename(&file, store::previous_of(&file)).unwrap();
        std::fs::write(&file, r#"{"file":"keys","version":99,"data":{"what":"keys this build can't read"}}"#).unwrap();
        let before = snapshot(&dir);
        let me = Identity::open(file.clone());
        assert_eq!(me.broken(), Some((file.as_path(), true)), "nothing is signed");
        assert_signs_nothing(&me, NEWER);
        assert_eq!(snapshot(&dir), before, "both files exactly as they were, and nothing new");
        assert!(store::notices_under(&dir).is_empty(), "nothing set aside");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_keys_file_the_system_wont_read_is_left_alone() {
        let dir = store::tests::folder("keys-unread");
        let file = dir.join("identity.json");
        let older = Identity::open(file.clone());
        older.finish_first_run("maya", true).unwrap();
        older.fresh("lamp person").unwrap();
        drop(older);
        std::fs::rename(&file, store::previous_of(&file)).unwrap();
        // A folder where the file should be: the system won't read it as one.
        std::fs::create_dir_all(file.join("inside")).unwrap();
        let before = snapshot(&dir);
        let me = Identity::open(file.clone());
        assert_eq!(me.broken(), Some((file.as_path(), false)));
        assert_signs_nothing(&me, UNREADABLE);
        assert_eq!(snapshot(&dir), before);
        assert!(file.join("inside").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_copy_of_the_app_without_its_folder_reads_your_keys_and_changes_nothing() {
        let dir = store::tests::folder("keys-untouched");
        let file = dir.join("identity.json");
        let mine = Identity::open(file.clone());
        let usual = mine.finish_first_run("maya", true).unwrap();
        mine.fresh("lamp person").unwrap();
        let before = snapshot(&dir);
        let other = Identity::untouched(&file, store::IN_USE);
        assert_eq!(other.usual().unwrap().key, usual.key, "the same you");
        assert_eq!(other.first_run(), FirstRun::Done);
        assert!(other.broken().is_none(), "the keys file is fine");
        assert_signs_nothing(&other, store::IN_USE);
        assert_eq!(snapshot(&dir), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn its_words_are_the_screens_words() {
        let screen = include_str!("../../src/strings.ts");
        for words in [UNREADABLE, NEWER, NOT_NAMED, NOT_ADULT, NO_NAME, store::IN_USE, store::UNCHECKED] {
            assert!(screen.contains(words), "src/strings.ts holds: {words}");
        }
    }

    /// Every file under a folder, at any depth.
    fn every_file(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { every_file(&path, out) } else { out.push(path) }
        }
    }

    fn holds(bytes: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty() && bytes.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn from_an_empty_folder_nobody_is_named_and_nothing_is_signed_until_a_name_is_typed() {
        let dir = store::tests::folder("keys-first-run");
        let file = dir.join("identity.json");
        // The login this Mac runs under is there to be read; the app never reads it.
        let login = std::env::var("USER").unwrap_or_default();
        let me = Identity::open(file.clone());
        assert_eq!(me.first_run(), FirstRun::New);
        assert!(me.personas().is_empty(), "no persona");
        assert_eq!(me.usual().unwrap_err(), NOT_NAMED);
        assert_signs_nothing(&me, NOT_NAMED);
        assert!(me.personas().is_empty(), "and still none: a fresh name needs you first");
        assert!(me.account().is_none());
        assert_eq!(me.finish_first_run("  ", true).unwrap_err(), NO_NAME);
        assert_eq!(me.finish_first_run("Ada", false).unwrap_err(), NOT_ADULT, "the 18-or-older line is confirmed on the page");
        assert!(!file.exists() && std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing written");
        let you = me.finish_first_run("  Ada  ", true).unwrap();
        assert_eq!((you.name.as_str(), you.usual), ("Ada", true), "the name as typed");
        assert_eq!(me.first_run(), FirstRun::Done);
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert_eq!(me.seal(&Actor::Persona("usual".into()), "room-1", &mut params).unwrap(), you.key, "now it signs");
        assert!(me.finish_first_run("someone else", true).is_err(), "once");
        if login.len() > 2 && !"Ada".contains(login.as_str()) {
            let mut files = Vec::new();
            every_file(&dir, &mut files);
            assert!(files.iter().all(|f| !holds(&std::fs::read(f).unwrap(), login.as_bytes())), "nothing here came from the login");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_account_names_this_macs_key_as_its_first_device() {
        let dir = store::tests::folder("keys-account");
        let file = dir.join("identity.json");
        let me = Identity::open(file.clone());
        let you = me.finish_first_run("Ada", true).unwrap();
        let (id, proof) = me.account().unwrap();
        let account = proof.check().unwrap();
        assert_eq!(account.id, id);
        assert_eq!(id, diverge_desktop_room::account::account_id(&proof.genesis), "the id is the genesis's digest");
        assert_eq!((account.sequence, account.devices), (1, vec![you.key.clone()]), "this Mac's key, the usual one");
        assert_ne!(account.root, you.key, "the root is not the device");
        let words = me.words().unwrap();
        assert_eq!(words.split(' ').count(), 12);
        let fresh = me.fresh("lamp person").unwrap();
        assert!(!proof.names(&fresh.key), "a fresh name is never in the account");
        // Kept across a restart, words and all, still unconfirmed.
        let again = Identity::open(file.clone());
        assert_eq!(again.account().map(|a| a.0), Some(id));
        assert_eq!(again.words().as_deref().map(String::as_str), Some(words.as_str()));
        assert!(!again.lock().account.as_ref().unwrap().words.confirmed);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_from_before_accounts_upgrades_in_place_and_its_rooms_still_take_its_seals() {
        use diverge_desktop_room::{Args, NoHost, Room};
        let dir = store::tests::folder("keys-v1");
        let file = dir.join("identity.json");
        let (usual, fresh, agent) = (Keypair::from_seed("maya"), Keypair::from_seed("lamp person"), Keypair::from_seed("agent site-fixes"));
        // A room hosted under the usual key, from before accounts.
        let room_key = Keypair::from_seed("room from before");
        let args: Args = serde_json::from_value(json!({
            "id": diverge_desktop_room::room_id("workshop", &usual.key()), "title": "Saturday Workshop", "kind": "board",
            "host_key": usual.key(), "host_name": "maya", "charter": "", "open_door": false, "continues": null,
            "room_key": room_key.key(), "at": Utc::now(), "sig": ""
        }))
        .unwrap();
        let mut room = Room::new(args.signed(&usual), room_key).unwrap();
        let rooms: BTreeMap<String, String> = [(room.id().to_owned(), "usual".to_owned()), ("elsewhere".to_owned(), "persona-2".to_owned())].into();
        let v1 = json!({ "file": "keys", "version": 1, "data": {
            "personas": [
                { "id": "usual", "name": "maya", "secret": usual.secret_hex(), "created": Utc::now(), "usual": true },
                { "id": "persona-2", "name": "lamp person", "secret": fresh.secret_hex(), "created": Utc::now(), "usual": false }
            ],
            "agents": { "site-fixes": { "secret": agent.secret_hex(), "persona": "usual" } },
            "rooms": rooms
        } })
        .to_string();
        std::fs::write(&file, &v1).unwrap();
        let me = Identity::open(file.clone());
        assert_eq!(me.first_run(), FirstRun::Earlier { name: "maya".into() }, "the page shows, naming the name the login gave");
        assert_signs_nothing(&me, NOT_NAMED);
        assert_eq!(std::fs::read_to_string(dir.join(BEFORE_ACCOUNTS)).unwrap(), v1, "the old file, kept as it was");
        assert_eq!(store::header(&file).map(|h| h.version), Some(store::KEYS.version), "upgraded in place");
        let you = me.finish_first_run("Maya R", true).unwrap();
        assert_eq!((you.key, you.name.as_str()), (usual.key(), "Maya R"), "the usual key, now this Mac's device key, under the name typed");
        assert!(me.account().unwrap().1.names(&usual.key()));
        // Nothing lost: the fresh name, the agent, the rooms.
        assert_eq!(me.persona("persona-2").map(|p| p.key), Some(fresh.key()));
        assert_eq!(me.in_room("elsewhere").map(|p| p.key), Some(fresh.key()));
        assert_eq!(me.agent_in("site-fixes", Some(room.id())).unwrap().key, agent.key());
        // And the room it hosts takes its seals, as it did.
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "still me" }).as_object().cloned().unwrap());
        me.seal(&me.you_in(room.id()), room.id(), &mut params).unwrap();
        room.call(params, &NoHost).unwrap();
        assert_eq!(room.moves().last().map(|m| m.by.clone()), Some(usual.key()));
        // The old file stays as it was after later saves too.
        me.fresh("another").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(BEFORE_ACCOUNTS)).unwrap(), v1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn after_first_run_the_words_are_only_in_identity_json_sealed_and_the_root_is_nowhere() {
        let dir = store::tests::folder("keys-scan");
        let file = dir.join("identity.json");
        let me = Identity::open(file.clone());
        me.finish_first_run("Ada", true).unwrap();
        // Everything that writes keys, once more.
        let fresh = me.fresh("lamp person").unwrap();
        me.set_room("room-1", &fresh.id);
        me.agent_in("site-fixes", Some("room-1")).unwrap();
        let mut params = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        me.seal(&Actor::Persona("usual".into()), "room-1", &mut params).unwrap();
        let words = me.words().unwrap();
        let seed = bip39::Mnemonic::parse(words.as_str()).unwrap().to_seed("");
        let root = diverge_desktop_room::account::root(&seed);
        let node = diverge_desktop_room::account::derive(&seed, diverge_desktop_room::account::ROOT_PATH).unwrap();
        assert_eq!(root.key(), me.account().unwrap().1.check().unwrap().root, "these are the words the root came from");
        let sealed = me.lock().account.as_ref().unwrap().words.sealed.clone();
        let mut files = Vec::new();
        every_file(&dir, &mut files);
        assert!(files.len() >= 3, "{files:?}");
        for f in &files {
            let bytes = std::fs::read(f).unwrap();
            let name = f.file_name().unwrap().to_string_lossy().into_owned();
            assert!(!holds(&bytes, words.as_bytes()), "{name} holds the words in plain");
            assert!(!holds(&bytes, words.replace(' ', "").as_bytes()), "{name} holds the words run together");
            for secret in [hex::encode(node.key), hex::encode(node.chain), hex::encode(seed), root.secret_hex()] {
                assert!(!holds(&bytes, secret.as_bytes()), "{name} holds the root, or the seed it came from");
            }
            assert!(!holds(&bytes, &node.key) && !holds(&bytes, &seed), "{name} holds the root's bytes");
            assert_eq!(holds(&bytes, sealed.as_bytes()), name == "identity.json" || name == "identity.json.bak", "{name}: the sealed words are in your keys file alone");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing in the app reads who this Mac says you are: not the login,
    /// not the machine's name. (Tests may, to check that.)
    #[test]
    fn nothing_in_the_app_reads_the_login_or_the_machines_name() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        every_file(&manifest.join("src"), &mut files);
        every_file(&manifest.join("../room/src"), &mut files);
        let looked_for = ["\"USER\"", "\"LOGNAME\"", "\"USERNAME\"", "hostname", "whoami", "home_dir", "getpwuid", "getlogin"];
        let mut read = 0;
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "rs")) {
            let text = std::fs::read_to_string(f).unwrap();
            // The app's own code: everything before its tests.
            let code = text.split("#[cfg(test)]\nmod tests").next().unwrap_or_default();
            let code = code.split("#[cfg(all(test").next().unwrap_or_default();
            for needle in looked_for {
                assert!(!code.contains(needle), "{} reads {needle}", f.display());
            }
            read += 1;
        }
        assert!(read > 20, "read the app's code: {read} files");
    }
}

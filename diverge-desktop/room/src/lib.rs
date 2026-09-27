//! The Diverge desktop app's room program. Ours, not Ronald's.
//!
//! A Space is a tool container a host runs on their own provider
//! (`containers::tools::run`); members join it by id (`tools::connect`) and
//! call its MCP tools. This crate is what runs inside: one room's verbs,
//! the seals that tell members apart, and the chained record that lets a
//! room be checked, restarted, or continued by someone else.
//!
//! The same program runs two ways: in-process, inside the app's stand-in
//! for rooms, and as a tool container image. Nothing in it knows which.

#[cfg(feature = "image")]
pub mod image;
pub mod room;
pub mod seal;

pub use room::{Args, Continues, Host, Kind, Member, Move, NoHost, Record, Room, key_mark};
pub use seal::{Key, Keypair, Seal, Statement, fresh_label, id_holds, room_id, seal_call, tether};

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use chrono::{DateTime, TimeDelta, Utc};
    use rmcp::model::{CallToolRequestParams, ResourceContents};
    use serde_json::{Value, json};

    use super::*;

    pub struct TestHost {
        pub keypair: Keypair,
        pub hires: Mutex<Vec<String>>,
    }

    impl Host for TestHost {
        fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
            Ok(Statement::make(&self.keypair, kind, body))
        }
        fn hire(&self, _room: &str, hire_id: &str, from: &str, agent: &str, what: &str, _pledge: Option<&str>) {
            self.hires.lock().unwrap().push(format!("{hire_id} {from} {agent} {what}"));
        }
    }

    pub struct Person {
        pub keypair: Keypair,
        pub counter: u64,
    }

    impl Person {
        pub fn new(seed: &str) -> Self {
            Person { keypair: Keypair::from_seed(seed), counter: 0 }
        }
        pub fn key(&self) -> String {
            self.keypair.key()
        }
        pub fn call(&mut self, room: &mut Room, host: &TestHost, verb: &'static str, args: Value) -> Result<String, String> {
            self.counter += 1;
            let mut params = CallToolRequestParams::new(verb).with_arguments(args.as_object().cloned().unwrap_or_default());
            seal_call(&self.keypair, room.id(), &mut params, self.counter);
            room.call(params, host)
                .map(|r| r.content.first().and_then(|c| c.as_text()).map(|t| t.text.clone()).unwrap_or_default())
                .map_err(|e| e.message.to_string())
        }
    }

    pub fn feed(room: &Room) -> Vec<Value> {
        let r = room.read(room::FEED).unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &r.contents[0] else { panic!() };
        serde_json::from_str(text).unwrap()
    }

    pub fn long_ago() -> DateTime<Utc> {
        Utc::now() - TimeDelta::days(30)
    }

    /// A room's settings, signed by its host, and the key the host made for it.
    pub fn settings(label: &str, host: &Person, title: &str, kind: Kind) -> (Args, Keypair) {
        let room_key = Keypair::from_seed(&format!("room {label}"));
        let args = Args {
            id: room_id(label, &host.key()),
            title: title.into(),
            kind,
            host_key: host.key(),
            host_name: "maya".into(),
            charter: format!("# {title}"),
            open_door: kind == Kind::Profile,
            continues: None,
            room_key: room_key.key(),
            at: long_ago(),
            sig: String::new(),
        }
        .signed(&host.keypair);
        (args, room_key)
    }

    pub fn board(maya: &Person) -> Room {
        let (args, key) = settings("board-1", maya, "Saturday Workshop", Kind::Board);
        Room::new(args, key).unwrap()
    }

    pub fn host_for(maya: &Person) -> TestHost {
        TestHost { keypair: maya.keypair.clone(), hires: Mutex::new(Vec::new()) }
    }

    #[test]
    fn a_room_opens_only_with_settings_its_host_signed() {
        let (maya, ren) = (Person::new("maya"), Person::new("ren"));
        let (args, key) = settings("board-1", &maya, "Saturday Workshop", Kind::Board);
        let mut changed = args.clone();
        changed.title = "Ren's workshop".into();
        assert!(Room::new(changed, key.clone()).is_err(), "settings changed after signing");
        let mut taken = args.clone();
        taken.host_key = ren.key();
        taken = taken.signed(&ren.keypair);
        assert!(Room::new(taken, key.clone()).is_err(), "ren can't run a room by maya's id");
        assert!(Room::new(args.clone(), Keypair::from_seed("another")).is_err(), "not the room's key");
        assert!(Room::new(args, key).is_ok());
    }

    #[test]
    fn members_speak_and_nobody_else_does() {
        let (mut maya, mut ren, mut stranger) = (Person::new("maya"), Person::new("ren"), Person::new("stranger"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        assert!(stranger.call(&mut room, &host, "show", json!({ "title": "hi" })).is_err(), "not admitted");
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        ren.call(&mut room, &host, "show", json!({ "title": "a lamp" })).unwrap();
        assert!(ren.call(&mut room, &host, "admit", json!({ "key": stranger.key(), "name": "x" })).is_err(), "host only");
        // The same sealed call can't be sent twice.
        ren.counter = 0;
        assert!(ren.call(&mut room, &host, "show", json!({ "title": "a lamp" })).is_err(), "no replays");
        // A call with no seal at all.
        let unsealed = CallToolRequestParams::new("show").with_arguments(json!({ "title": "x" }).as_object().cloned().unwrap());
        assert!(room.call(unsealed, &host).is_err());
        let shown = feed(&room).into_iter().find(|m| m["kind"] == "show").unwrap();
        assert_eq!(shown["author"], "ren");
        assert_eq!(shown["by"], ren.key());
    }

    #[test]
    fn an_agent_comes_in_tethered_to_its_person() {
        let (mut maya, ren) = (Person::new("maya"), Person::new("ren"));
        let mut helper = Person::new("ren's helper");
        let host = host_for(&maya);
        let mut room = board(&maya);
        let untethered = json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key() });
        assert!(maya.call(&mut room, &host, "admit", untethered).is_err(), "no tether");
        let t = tether(&ren.keypair, &helper.key(), "helper");
        assert!(maya.call(&mut room, &host, "admit", json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key(), "tether": t })).is_err(), "its person isn't a member yet");
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        let forged = tether(&maya.keypair, &helper.key(), "helper");
        assert!(maya.call(&mut room, &host, "admit", json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key(), "tether": forged })).is_err(), "someone else's tether");
        maya.call(&mut room, &host, "admit", json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key(), "tether": t })).unwrap();
        helper.call(&mut room, &host, "show", json!({ "title": "done" })).unwrap();
        let shown = feed(&room).into_iter().find(|m| m["kind"] == "show").unwrap();
        assert_eq!(shown["agent_of"], "ren", "the room knows whose agent it is");
    }

    #[test]
    fn a_task_ends_in_one_receipt_the_host_sealed_and_both_sides_settle_once() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "post_task", json!({ "title": "Fix the lamp", "spec": "It turns on.", "pledge": "a coffee" })).unwrap();
        assert!(ren.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": true })).is_err(), "nothing to settle yet");
        ren.call(&mut room, &host, "claim", json!({ "task_id": "task-2" })).unwrap();
        assert!(maya.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "x" })).is_err(), "only who claimed it delivers");
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch.", "files": ["lamp/switch.jpg"] })).unwrap();
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch, and a new bulb." })).unwrap();
        assert!(ren.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).is_err(), "only who posted it accepts");
        maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).unwrap();
        assert!(ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "again" })).is_err(), "done is done");
        assert!(maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).is_err(), "one receipt");
        let moves = feed(&room);
        assert_eq!(moves.iter().filter(|m| m["kind"] == "receipt").count(), 1);
        let task = moves.iter().find(|m| m["id"] == "task-2").unwrap();
        assert_eq!(task["state"], "done");
        let receipt = moves.iter().find(|m| m["kind"] == "receipt").unwrap();
        let statement: Statement = serde_json::from_value(receipt["fields"]["statement"].clone()).unwrap();
        assert!(statement.holds());
        assert_eq!(statement.key, maya.key(), "sealed by the host");
        assert_eq!(statement.field("to_person"), Some(ren.key().as_str()));
        ren.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": true })).unwrap();
        assert!(ren.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": false })).is_err(), "each side says once");
        maya.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": false, "note": "coffee next week" })).unwrap();
        let task = feed(&room).into_iter().find(|m| m["id"] == "task-2").unwrap();
        assert_eq!(task["fields"]["doer_says"]["agree"], true);
        assert_eq!(task["fields"]["poster_says"]["agree"], false, "both reports stand");
    }

    #[test]
    fn a_removed_member_is_refused_and_their_agent_goes_with_them_in_the_same_move() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let mut helper = Person::new("helper");
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        let t = tether(&ren.keypair, &helper.key(), "helper");
        maya.call(&mut room, &host, "admit", json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key(), "tether": t })).unwrap();
        maya.call(&mut room, &host, "remove", json!({ "key": ren.key(), "reason": "spam" })).unwrap();
        assert!(ren.call(&mut room, &host, "show", json!({ "title": "x" })).unwrap_err().contains("removed"));
        assert!(helper.call(&mut room, &host, "show", json!({ "title": "x" })).is_err());
        let removal = room.moves().last().unwrap();
        assert_eq!(removal.fields["also"], json!([helper.key()]), "the record says the agent went too");
        assert!(Room::check(&room.record()).unwrap().member(&helper.key()).unwrap().removed, "and a replay agrees");
    }

    #[test]
    fn someone_unlisted_is_a_mark_in_the_record_until_they_act() {
        let (mut maya, mut ren, mut ada) = (Person::new("maya"), Person::new("ren"), Person::new("ada"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        let id = room.id().to_owned();
        assert!(maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren", "listed": false })).is_err(), "never by key");
        maya.call(&mut room, &host, "admit", json!({ "key_mark": key_mark(&id, &ren.key()), "name": "ren", "listed": false })).unwrap();
        maya.call(&mut room, &host, "admit", json!({ "key_mark": key_mark(&id, &ada.key()), "name": "ada", "listed": false })).unwrap();
        let record = serde_json::to_string(&room.record()).unwrap();
        assert!(!record.contains(&ren.key()), "the record doesn't name ren's key");
        assert!(room.may_read(&ren.key()));
        let listed = room.read(room::MEMBERS).unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &listed.contents[0] else { panic!() };
        assert!(!text.contains("\"ren\""), "ren chose not to be listed");
        ren.call(&mut room, &host, "show", json!({ "title": "a radio" })).unwrap();
        assert_eq!(feed(&room).into_iter().find(|m| m["kind"] == "show").unwrap()["author"], "ren", "once they act, their moves are theirs");
        assert!(Room::check(&room.record()).is_ok(), "and a copy replays it");
        // ada never acted: she's removed by her mark, and her key is never named.
        maya.call(&mut room, &host, "remove", json!({ "key": key_mark(&id, &ada.key()) })).unwrap();
        assert!(!serde_json::to_string(&room.record()).unwrap().contains(&ada.key()));
        assert!(!room.may_read(&ada.key()));
        assert!(ada.call(&mut room, &host, "show", json!({ "title": "x" })).is_err());
        assert!(Room::check(&room.record()).is_ok());
    }

    #[test]
    fn a_room_restarts_from_its_record() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "post_task", json!({ "title": "Fix the lamp", "spec": "It turns on." })).unwrap();
        ren.call(&mut room, &host, "claim", json!({ "task_id": "task-2" })).unwrap();
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch." })).unwrap();
        maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).unwrap();
        maya.call(&mut room, &host, "set_charter", json!({ "text": "# v2" })).unwrap();
        let key = Keypair::from_seed("room board-1");
        let mut again = Room::from_record(room.record(), Some(key)).unwrap();
        assert_eq!(feed(&again), feed(&room), "the same room");
        assert_eq!(again.charter(), "# v2");
        // A later call must use a fresh counter even after a restart.
        ren.counter -= 1;
        assert!(ren.call(&mut again, &host, "show", json!({ "title": "late" })).is_err());
        ren.counter += 1;
        ren.call(&mut again, &host, "show", json!({ "title": "late" })).unwrap();
        // A copy checks, but can't make moves.
        let mut copy = Room::check(&room.record()).unwrap();
        assert!(ren.call(&mut copy, &host, "show", json!({ "title": "x" })).is_err());
    }

    #[test]
    fn a_member_continues_a_room_whose_host_is_gone_and_its_past_reads_as_it_was() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "post_task", json!({ "title": "Fix the lamp", "spec": "It turns on." })).unwrap();
        ren.call(&mut room, &host, "claim", json!({ "task_id": "task-2" })).unwrap();
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch." })).unwrap();
        maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).unwrap();
        let old = room.record();
        let ren_host = host_for(&ren);
        let (mut args, key) = settings("board-2", &ren, "Saturday Workshop, continued", Kind::Board);
        args.host_name = "ren".into();
        args.continues = Some(Continues { room: room.id().into(), title: "Saturday Workshop".into(), last: room.last_hash() });
        let args = args.signed(&ren.keypair);
        let mut next = Room::from_record(Record { args: args.clone(), before: Some(Box::new(old.clone())), moves: Vec::new() }, Some(key.clone())).unwrap();
        let moves = feed(&next);
        assert_eq!(moves.len(), old.moves.len(), "the old moves come along");
        assert_eq!(moves[1]["fields"]["from"], "Saturday Workshop");
        assert_eq!(moves.iter().find(|m| m["id"] == "task-2").unwrap()["state"], "done", "a finished task reads finished");
        ren.counter = 0;
        ren.call(&mut next, &ren_host, "show", json!({ "title": "still here" })).unwrap();
        // It restarts like any room.
        assert!(Room::from_record(next.record(), Some(key.clone())).is_ok());
        let mut cut = old.clone();
        cut.moves.pop();
        assert!(Room::from_record(Record { args, before: Some(Box::new(cut)), moves: Vec::new() }, Some(key)).is_err(), "a history that stops short");
    }

    #[test]
    fn a_visitor_hires_through_a_profile_and_the_host_decides_once() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let (args, key) = settings("profile", &maya, "maya", Kind::Profile);
        let mut profile = Room::new(args, key).unwrap();
        maya.call(&mut profile, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        assert!(ren.call(&mut profile, &host, "show", json!({ "title": "x" })).is_err(), "only the owner shows here");
        ren.call(&mut profile, &host, "leave_note", json!({ "body": "love the lamp" })).unwrap();
        ren.call(&mut profile, &host, "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee" })).unwrap();
        assert_eq!(host.hires.lock().unwrap().len(), 1, "the host hears about it");
        assert!(maya.call(&mut profile, &host, "deliver_hire", json!({ "hire_id": "hire-3", "summary": "x" })).is_err(), "not taken yet");
        maya.call(&mut profile, &host, "answer_hire", json!({ "hire_id": "hire-3", "take": true })).unwrap();
        assert!(maya.call(&mut profile, &host, "answer_hire", json!({ "hire_id": "hire-3", "take": false })).is_err(), "answered once");
        maya.call(&mut profile, &host, "deliver_hire", json!({ "hire_id": "hire-3", "summary": "Two broken links.", "files": ["for-ren/links.md"] })).unwrap();
        assert_eq!(feed(&profile).into_iter().find(|m| m["id"] == "hire-3").unwrap()["state"], "delivered");
        // A receipt from elsewhere, issued to maya by that room's host, pins.
        let issuer = Keypair::from_seed("some host");
        let workshop = room_id("workshop", &issuer.key());
        let mine = Statement::make(&issuer, "receipt", json!({ "title": "Fix the lamp", "room": workshop, "room_title": "Saturday Workshop", "to_person": maya.key() }));
        let theirs = Statement::make(&issuer, "receipt", json!({ "title": "x", "room": workshop, "to_person": ren.key() }));
        let elsewhere = Statement::make(&issuer, "receipt", json!({ "title": "x", "room": room_id("workshop", &ren.key()), "to_person": maya.key() }));
        let self_made = Statement::make(&maya.keypair, "receipt", json!({ "title": "x", "room": room_id("mine", &maya.key()), "to_person": maya.key() }));
        maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": mine })).unwrap();
        assert!(maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": theirs })).is_err(), "issued to someone else");
        assert!(maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": elsewhere })).is_err(), "not that room's host");
        assert!(maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": self_made })).is_err(), "sealed by yourself");
    }
}

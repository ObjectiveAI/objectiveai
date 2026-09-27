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

pub use room::{Args, Continues, Host, Kind, Member, Move, Room, check_record};
pub use seal::{Key, Keypair, Seal, Statement, seal_call, tether};

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rmcp::model::{CallToolRequestParams, ResourceContents};
    use serde_json::{Value, json};

    use super::*;

    struct TestHost {
        keypair: Keypair,
        hires: Mutex<Vec<String>>,
    }

    impl Host for TestHost {
        fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
            Ok(Statement::make(&self.keypair, kind, body))
        }
        fn hire(&self, _room: &str, hire_id: &str, from: &str, agent: &str, what: &str, _pledge: Option<&str>) {
            self.hires.lock().unwrap().push(format!("{hire_id} {from} {agent} {what}"));
        }
    }

    struct Person {
        keypair: Keypair,
        counter: u64,
    }

    impl Person {
        fn new(seed: &str) -> Self {
            Person { keypair: Keypair::from_seed(seed), counter: 0 }
        }
        fn key(&self) -> String {
            self.keypair.key()
        }
        fn call(&mut self, room: &mut Room, host: &TestHost, verb: &'static str, args: Value) -> Result<String, String> {
            self.counter += 1;
            let mut params = CallToolRequestParams::new(verb).with_arguments(args.as_object().cloned().unwrap_or_default());
            seal_call(&self.keypair, room.id(), &mut params, self.counter);
            room.call(params, host)
                .map(|r| r.content.first().and_then(|c| c.as_text()).map(|t| t.text.clone()).unwrap_or_default())
                .map_err(|e| e.message.to_string())
        }
    }

    fn feed(room: &Room) -> Vec<Value> {
        let r = room.read(room::FEED).unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &r.contents[0] else { panic!() };
        serde_json::from_str(text).unwrap()
    }

    fn board(maya: &Person) -> Room {
        Room::new(Args {
            id: "board-1".into(),
            title: "Saturday Workshop".into(),
            kind: Kind::Board,
            host_key: maya.key(),
            host_name: "maya".into(),
            charter: "# Saturday Workshop".into(),
            open_door: false,
            continues: None,
        })
    }

    fn host_for(maya: &Person) -> TestHost {
        TestHost { keypair: maya.keypair.clone(), hires: Mutex::new(Vec::new()) }
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
    fn a_task_ends_in_a_receipt_the_host_sealed_and_both_sides_settle() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "post_task", json!({ "title": "Fix the lamp", "spec": "It turns on.", "pledge": "a coffee" })).unwrap();
        ren.call(&mut room, &host, "claim", json!({ "task_id": "task-2" })).unwrap();
        assert!(maya.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "x" })).is_err(), "only who claimed it delivers");
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch.", "files": ["lamp/switch.jpg"] })).unwrap();
        assert!(ren.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).is_err(), "only who posted it accepts");
        maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).unwrap();
        let moves = feed(&room);
        let task = moves.iter().find(|m| m["id"] == "task-2").unwrap();
        assert_eq!(task["state"], "done");
        let receipt = moves.iter().find(|m| m["kind"] == "receipt").unwrap();
        let statement: Statement = serde_json::from_value(receipt["fields"]["statement"].clone()).unwrap();
        assert!(statement.holds());
        assert_eq!(statement.key, maya.key(), "sealed by the host");
        assert_eq!(statement.field("to_person"), Some(ren.key().as_str()));
        ren.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": true })).unwrap();
        maya.call(&mut room, &host, "settle", json!({ "task_id": "task-2", "agree": false, "note": "coffee next week" })).unwrap();
        let task = feed(&room).into_iter().find(|m| m["id"] == "task-2").unwrap();
        assert_eq!(task["fields"]["doer_says"]["agree"], true);
        assert_eq!(task["fields"]["poster_says"]["agree"], false, "both reports stand");
    }

    #[test]
    fn a_removed_member_is_refused_and_their_agent_goes_with_them() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let mut helper = Person::new("helper");
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren", "listed": false })).unwrap();
        let t = tether(&ren.keypair, &helper.key(), "helper");
        maya.call(&mut room, &host, "admit", json!({ "key": helper.key(), "name": "helper", "is_agent": true, "agent_of": ren.key(), "tether": t })).unwrap();
        let listed = room.read(room::MEMBERS).unwrap();
        let ResourceContents::TextResourceContents { text, .. } = &listed.contents[0] else { panic!() };
        assert!(!text.contains("\"ren\""), "ren chose not to be listed");
        maya.call(&mut room, &host, "remove", json!({ "key": ren.key(), "reason": "spam" })).unwrap();
        assert!(ren.call(&mut room, &host, "show", json!({ "title": "x" })).unwrap_err().contains("removed"));
        assert!(helper.call(&mut room, &host, "show", json!({ "title": "x" })).is_err());
    }

    #[test]
    fn a_room_rebuilds_from_its_record_and_a_changed_record_is_caught() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "post_task", json!({ "title": "Fix the lamp", "spec": "It turns on." })).unwrap();
        ren.call(&mut room, &host, "claim", json!({ "task_id": "task-2" })).unwrap();
        ren.call(&mut room, &host, "deliver", json!({ "task_id": "task-2", "summary": "New switch." })).unwrap();
        maya.call(&mut room, &host, "accept", json!({ "task_id": "task-2" })).unwrap();
        maya.call(&mut room, &host, "set_charter", json!({ "text": "# v2" })).unwrap();
        let record = room.moves().to_vec();
        let again = Room::from_record(room.args.clone(), Vec::new(), &record, &host).unwrap();
        assert_eq!(feed(&again), feed(&room), "the same room");
        assert_eq!(again.charter(), "# v2");
        // A later call must use a fresh counter even after a restart.
        let mut again = again;
        ren.counter -= 1;
        assert!(ren.call(&mut again, &host, "show", json!({ "title": "late" })).is_err());
        let mut tampered = record.clone();
        tampered[1].body = "It turns on and plays music.".into();
        assert!(Room::from_record(room.args.clone(), Vec::new(), &tampered, &host).is_err());
        let mut dropped = record.clone();
        dropped.remove(2);
        assert!(Room::from_record(room.args.clone(), Vec::new(), &dropped, &host).is_err(), "a missing move breaks the chain");
    }

    #[test]
    fn a_member_continues_a_room_whose_host_is_gone() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        ren.call(&mut room, &host, "show", json!({ "title": "a lamp" })).unwrap();
        let old = room.moves().to_vec();
        let ren_host = host_for(&ren);
        let args = Args {
            id: "board-2".into(),
            title: "Saturday Workshop, continued".into(),
            kind: Kind::Board,
            host_key: ren.key(),
            host_name: "ren".into(),
            charter: room.charter().into(),
            open_door: false,
            continues: Some(Continues { room: "board-1".into(), title: "Saturday Workshop".into(), last: room.last_hash() }),
        };
        let mut next = Room::from_record(args.clone(), old.clone(), &[], &ren_host).unwrap();
        let moves = feed(&next);
        assert_eq!(moves.len(), old.len(), "the old moves come along");
        assert_eq!(moves[1]["fields"]["from"], "Saturday Workshop");
        ren.call(&mut next, &ren_host, "show", json!({ "title": "still here" })).unwrap();
        let mut cut = old.clone();
        cut.pop();
        assert!(Room::from_record(args, cut, &[], &ren_host).is_err(), "a history that stops short");
    }

    #[test]
    fn a_visitor_hires_through_a_profile_and_the_host_decides() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut profile = Room::new(Args {
            id: "profile-maya".into(),
            title: "maya".into(),
            kind: Kind::Profile,
            host_key: maya.key(),
            host_name: "maya".into(),
            charter: "# maya".into(),
            open_door: true,
            continues: None,
        });
        maya.call(&mut profile, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        assert!(ren.call(&mut profile, &host, "show", json!({ "title": "x" })).is_err(), "only the owner shows here");
        ren.call(&mut profile, &host, "leave_note", json!({ "body": "love the lamp" })).unwrap();
        ren.call(&mut profile, &host, "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee" })).unwrap();
        assert_eq!(host.hires.lock().unwrap().len(), 1, "the host hears about it");
        assert!(maya.call(&mut profile, &host, "deliver_hire", json!({ "hire_id": "hire-3", "summary": "x" })).is_err(), "not taken yet");
        maya.call(&mut profile, &host, "answer_hire", json!({ "hire_id": "hire-3", "take": true })).unwrap();
        maya.call(&mut profile, &host, "deliver_hire", json!({ "hire_id": "hire-3", "summary": "Two broken links.", "files": ["for-ren/links.md"] })).unwrap();
        assert_eq!(feed(&profile).into_iter().find(|m| m["id"] == "hire-3").unwrap()["state"], "delivered");
        // A receipt from elsewhere, issued to maya, pins; one issued to ren does not.
        let issuer = Keypair::from_seed("some host");
        let mine = Statement::make(&issuer, "receipt", json!({ "title": "Fix the lamp", "room_title": "Saturday Workshop", "to_person": maya.key() }));
        let theirs = Statement::make(&issuer, "receipt", json!({ "title": "x", "to_person": ren.key() }));
        maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": mine })).unwrap();
        assert!(maya.call(&mut profile, &host, "pin_receipt", json!({ "statement": theirs })).is_err());
    }
}

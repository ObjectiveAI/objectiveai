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

pub mod account;
pub mod envelope;
#[cfg(feature = "image")]
pub mod image;
pub mod room;
pub mod seal;

pub use room::{Args, Continues, Host, Keeper, Kind, Member, Move, NOT_A_MEMBER, NoHost, Record, Room, Rules, Standing, invite_lock, key_mark, receipt_issuer};
pub use seal::{Key, Keypair, Seal, Statement, account_id_holds, account_room_id, fresh_label, id_holds, room_id, seal_call, tether};

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
        fn hire(&self, _room: &str, hire_id: &str, from: &str, ask: &Value) {
            let line = match ask.get("sealed") {
                Some(_) => format!("{hire_id} {from} sealed"),
                None => format!("{hire_id} {from} {} {}", ask["agent"].as_str().unwrap_or_default(), ask["what"].as_str().unwrap_or_default()),
            };
            self.hires.lock().unwrap().push(line);
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
            rules: 1,
            host_account: None,
            keepers: Vec::new(),
            notes_key: None,
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
    fn where_someone_stands_is_read_from_the_record() {
        let (mut maya, mut ren, mut ada, mut sam, bo) = (Person::new("maya"), Person::new("ren"), Person::new("ada"), Person::new("sam"), Person::new("bo"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        let id = room.id().to_owned();
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "admit", json!({ "key_mark": key_mark(&id, &ada.key()), "name": "ada", "listed": false })).unwrap();
        maya.call(&mut room, &host, "admit", json!({ "key_mark": key_mark(&id, &sam.key()), "name": "sam", "listed": false })).unwrap();
        sam.call(&mut room, &host, "show", json!({ "title": "a stool" })).unwrap();
        maya.call(&mut room, &host, "admit", json!({ "key": bo.key(), "name": "bo" })).unwrap();
        maya.call(&mut room, &host, "remove", json!({ "key": bo.key() })).unwrap();
        let copy = Room::check(&room.record()).unwrap();
        for r in [&room, &copy] {
            assert_eq!(r.standing(&ren.key()), Standing::Member, "listed");
            assert_eq!(r.standing(&sam.key()), Standing::Member, "unlisted, and acted since");
            assert_eq!(r.standing(&ada.key()), Standing::Unlisted { mark: key_mark(&id, &ada.key()) }, "unlisted, never acted: by mark");
            assert_eq!(r.standing(&bo.key()), Standing::Removed);
            assert_eq!(r.standing(&Person::new("a stranger").key()), Standing::Stranger);
            assert_eq!(r.standing(&maya.key()), Standing::Member, "the host");
        }
        // Someone let in unlisted and removed before they ever acted reads as removed, by mark.
        maya.call(&mut room, &host, "remove", json!({ "key": key_mark(&id, &ada.key()) })).unwrap();
        assert_eq!(room.standing(&ada.key()), Standing::Removed);
        assert_eq!(Room::check(&room.record()).unwrap().standing(&ada.key()), Standing::Removed, "and a copy agrees");
        assert!(ada.call(&mut room, &host, "show", json!({ "title": "x" })).is_err());
        let _ = (&mut ren, &mut sam);
    }

    #[test]
    fn under_rules_two_standing_is_by_account_and_by_any_current_device() {
        let (mut maya, ren_mac, desktop) = (Person::new("maya's mac"), Person::new("ren's laptop"), Person::new("ren's desktop"));
        let root = Keypair::from_seed("maya's root");
        let mine = account::Proof::first(&root, long_ago(), &[maya.key()]);
        let ren_root = Keypair::from_seed("ren's root");
        let ren = account::Proof::first(&ren_root, long_ago(), &[ren_mac.key()]);
        let room_key = Keypair::from_seed("room v2");
        let args = Args {
            id: seal::account_room_id("board", &mine.id()),
            title: "Saturday Workshop".into(),
            kind: Kind::Board,
            host_key: maya.key(),
            host_name: "maya".into(),
            charter: "#".into(),
            open_door: false,
            continues: None,
            room_key: room_key.key(),
            at: long_ago(),
            rules: 2,
            host_account: Some(mine.clone()),
            keepers: Vec::new(),
            notes_key: None,
            sig: String::new(),
        }
        .signed(&maya.keypair);
        let host = host_for(&maya);
        let mut room = Room::new(args, room_key).unwrap();
        maya.call(&mut room, &host, "admit", json!({ "account": ren, "name": "ren" })).unwrap();
        assert_eq!(room.standing(&ren.id()), Standing::Member);
        assert_eq!(room.standing(&ren_mac.key()), Standing::Member, "a current device stands for its account");
        assert_eq!(room.standing(&desktop.key()), Standing::Stranger, "a device the room hasn't been shown");
        assert_eq!(room.standing(&mine.id()), Standing::Member, "the host, by account");
        maya.call(&mut room, &host, "remove", json!({ "key": ren_mac.key() })).unwrap();
        assert_eq!(room.standing(&ren.id()), Standing::Removed, "removed by a device, it's the account that's out");
        assert_eq!(room.standing(&ren_mac.key()), Standing::Removed);
    }

    #[test]
    fn an_offer_taken_on_a_board_becomes_a_task_that_ends_in_a_receipt() {
        let (mut maya, mut ren) = (Person::new("maya"), Person::new("ren"));
        let host = host_for(&maya);
        let mut room = board(&maya);
        maya.call(&mut room, &host, "admit", json!({ "key": ren.key(), "name": "ren" })).unwrap();
        maya.call(&mut room, &host, "ask", json!({ "what": "Fix the lamp" })).unwrap();
        ren.call(&mut room, &host, "offer", json!({ "ask_id": "ask-2", "body": "New switch, Saturday." })).unwrap();
        assert!(ren.call(&mut room, &host, "take_offer", json!({ "offer_id": "offer-3" })).is_err(), "only who asked takes");
        maya.call(&mut room, &host, "take_offer", json!({ "offer_id": "offer-3" })).unwrap();
        assert!(ren.call(&mut room, &host, "offer", json!({ "ask_id": "ask-2", "body": "me too" })).is_err(), "taken asks take no offers");
        let moves = feed(&room);
        assert_eq!(moves.iter().find(|m| m["id"] == "ask-2").unwrap()["state"], "taken");
        let task = moves.iter().find(|m| m["kind"] == "task").unwrap().clone();
        assert_eq!((task["state"].as_str(), task["body"].as_str()), (Some("claimed"), Some("New switch, Saturday.")));
        let id = task["id"].as_str().unwrap().to_owned();
        ren.call(&mut room, &host, "deliver", json!({ "task_id": id, "summary": "Done." })).unwrap();
        maya.call(&mut room, &host, "accept", json!({ "task_id": id })).unwrap();
        assert!(feed(&room).iter().any(|m| m["kind"] == "receipt"));
        maya.call(&mut room, &host, "ask", json!({ "what": "A ladder" })).unwrap();
        let ask = feed(&room).into_iter().rev().find(|m| m["kind"] == "ask").unwrap()["id"].as_str().unwrap().to_owned();
        maya.call(&mut room, &host, "close_ask", json!({ "ask_id": ask, "note": "found one" })).unwrap();
        assert!(ren.call(&mut room, &host, "offer", json!({ "ask_id": ask, "body": "mine?" })).is_err(), "closed");
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

    /// A profile under rules 2, hosted by maya's account, sealing what visitors leave to her notes key.
    pub fn sealed_profile(maya: &Person, notes: &envelope::OpenKey) -> (Room, account::Proof) {
        let root = Keypair::from_seed("maya's root");
        let mine = account::Proof::first(&root, long_ago(), &[maya.key()]);
        let room_key = Keypair::from_seed("profile v2");
        let args = Args {
            id: seal::account_room_id("profile", &mine.id()),
            title: "maya".into(),
            kind: Kind::Profile,
            host_key: maya.key(),
            host_name: "maya".into(),
            charter: "#".into(),
            open_door: true,
            continues: None,
            room_key: room_key.key(),
            at: long_ago(),
            rules: 2,
            host_account: Some(mine.clone()),
            keepers: Vec::new(),
            notes_key: Some(notes.public()),
            sig: String::new(),
        }
        .signed(&maya.keypair);
        (Room::new(args, room_key).unwrap(), mine)
    }

    /// A call with its words sealed to `readers`, as the app sends it.
    pub fn sealed_call(who: &mut Person, room: &mut Room, host: &TestHost, verb: &'static str, args: Value, readers: &[String]) -> Result<String, String> {
        let sealed = envelope::seal_args(room.id(), verb, args.as_object().unwrap(), &who.key(), readers).unwrap();
        who.call(room, host, verb, Value::Object(sealed))
    }

    fn one_device(person: &Person, root: &str) -> account::Proof {
        account::Proof::first(&Keypair::from_seed(root), long_ago(), &[person.key()])
    }

    #[test]
    fn what_visitors_leave_on_a_profile_stays_between_them_and_its_owner() {
        let (mut maya, mut ren, mut ada) = (Person::new("maya's mac"), Person::new("ren's laptop"), Person::new("ada's pc"));
        let notes = envelope::OpenKey::from_seed(b"maya's words");
        let host = host_for(&maya);
        let (mut profile, _) = sealed_profile(&maya, &notes);
        let id = profile.id().to_owned();
        maya.call(&mut profile, &host, "admit", json!({ "account": one_device(&ren, "ren's root"), "name": "ren" })).unwrap();
        maya.call(&mut profile, &host, "admit", json!({ "account": one_device(&ada, "ada's root"), "name": "ada" })).unwrap();
        let ren_reads = envelope::OpenKey::for_author(&ren.keypair, &id);
        let to_maya = [notes.public(), ren_reads.public()];

        // Nothing a visitor leaves reaches the room in the clear.
        assert!(ren.call(&mut profile, &host, "leave_note", json!({ "body": "love the lamp" })).is_err(), "a plain note");
        assert!(ren.call(&mut profile, &host, "hire", json!({ "agent": "site-fixes", "what": "check my links" })).is_err(), "a plain hire");
        let mixed = envelope::seal_args(&id, "hire", json!({ "what": "check my links" }).as_object().unwrap(), &ren.key(), &to_maya).unwrap();
        let mut mixed = Value::Object(mixed);
        mixed["agent"] = json!("site-fixes");
        assert!(ren.call(&mut profile, &host, "hire", mixed).unwrap_err().contains("travels sealed"), "nor any part of one");
        sealed_call(&mut ren, &mut profile, &host, "leave_note", json!({ "body": "love the lamp" }), &to_maya).unwrap();
        let asked = sealed_call(&mut ren, &mut profile, &host, "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee", "reply_to": ren_reads.public() }), &to_maya).unwrap();
        let hire = asked.rsplit_once('(').and_then(|(_, h)| h.strip_suffix(')')).unwrap().to_owned();
        assert_eq!(host.hires.lock().unwrap().last().unwrap(), &format!("{hire} ren sealed"), "the host hears of it, sealed");

        // The owner opens them; so does their author; ada can't.
        let moves = feed(&profile);
        let note = moves.iter().find(|m| m["kind"] == "note").unwrap();
        let asked = moves.iter().find(|m| m["kind"] == "hire").unwrap();
        let open = |m: &Value, verb: &str, key: &envelope::OpenKey| envelope::open(&envelope::shape(&m["fields"]["sealed"]).unwrap(), key, &id, verb, m["by"].as_str().unwrap());
        assert_eq!(open(note, "leave_note", &notes).unwrap()["body"], "love the lamp");
        assert_eq!(open(asked, "hire", &notes).unwrap()["pledge"], "a coffee");
        assert_eq!(open(note, "leave_note", &ren_reads).unwrap()["body"], "love the lamp", "ren reads his own");
        let ada_reads = envelope::OpenKey::for_author(&ada.keypair, &id);
        assert!(open(note, "leave_note", &ada_reads).is_none() && open(asked, "hire", &ada_reads).is_none(), "ada reads neither");

        // Taken and delivered, sealed to the owner and whoever asked.
        let reply_to = open(asked, "hire", &notes).unwrap()["reply_to"].as_str().unwrap().to_owned();
        let back = [notes.public(), reply_to];
        assert!(maya.call(&mut profile, &host, "answer_hire", json!({ "hire_id": hire, "take": true, "note": "Saturday" })).is_err(), "an answer's note is sealed too");
        sealed_call(&mut maya, &mut profile, &host, "answer_hire", json!({ "hire_id": hire, "take": true, "note": "Saturday" }), &back).unwrap();
        assert!(maya.call(&mut profile, &host, "deliver_hire", json!({ "hire_id": hire, "summary": "Two broken links." })).is_err(), "a plain result");
        sealed_call(&mut maya, &mut profile, &host, "deliver_hire", json!({ "hire_id": hire, "summary": "Two broken links.", "result": "/old-page and /zine are broken." }), &back).unwrap();
        let moves = feed(&profile);
        let result = moves.iter().find(|m| m["kind"] == "hire_delivery").unwrap();
        assert_eq!(moves.iter().find(|m| m["id"] == hire.as_str()).unwrap()["state"], "delivered", "its state is the room's to keep, in the clear");
        assert_eq!(open(result, "deliver_hire", &ren_reads).unwrap()["result"], "/old-page and /zine are broken.", "ren reads the result");
        assert_eq!(open(result, "deliver_hire", &notes).unwrap()["summary"], "Two broken links.", "so does maya");
        assert!(open(result, "deliver_hire", &ada_reads).is_none(), "ada doesn't");

        // ada can't change any of it, answer it in the open, or pass ren's words off as hers.
        let note_id = note["id"].as_str().unwrap().to_owned();
        assert!(ada.call(&mut profile, &host, "withdraw", json!({ "move_id": note_id })).is_err());
        assert!(ada.call(&mut profile, &host, "reply", json!({ "move_id": note_id, "body": "me too" })).is_err());
        let copied = json!({ "sealed": note["fields"]["sealed"] });
        ada.call(&mut profile, &host, "leave_note", copied).unwrap();
        let theirs = feed(&profile).into_iter().rev().find(|m| m["kind"] == "note").unwrap();
        assert!(open(&theirs, "leave_note", &notes).is_none(), "an envelope carried to another author opens for nobody");
        let ada_reads_to = [notes.public(), ada_reads.public()];
        sealed_call(&mut ada, &mut profile, &host, "leave_note", json!({ "body": "hello from ada" }), &ada_reads_to).unwrap();
        let mut changed = profile.record();
        let ada_note = changed.moves.last().unwrap().args.clone();
        changed.moves.iter_mut().find(|m| m.id == note_id).unwrap().args = ada_note;
        assert!(Room::check(&changed).is_err(), "nor can ren's note be swapped in a copy");

        // A copy held by someone who isn't the owner holds only ciphertext.
        let copy = serde_json::to_string(&Room::check(&profile.record()).unwrap().record()).unwrap();
        for words in ["love the lamp", "check my links", "site-fixes", "a coffee", "Saturday", "Two broken links", "/old-page"] {
            assert!(!copy.contains(words), "a copy holds \"{words}\" in the clear");
        }
        let served = serde_json::to_string(&feed(&profile)).unwrap();
        assert!(!served.contains("love the lamp") && !served.contains("a coffee"), "nor does the feed every member is served");
    }

    #[test]
    fn only_a_profile_under_rules_two_names_a_notes_key() {
        let maya = Person::new("maya's mac");
        let notes = envelope::OpenKey::from_seed(b"maya's words");
        let (profile, _) = sealed_profile(&maya, &notes);
        let mut args = profile.args.clone();
        args.kind = Kind::Board;
        assert!(args.signed(&maya.keypair).holds().is_err(), "a board with one");
        let mut args = profile.args.clone();
        args.notes_key = Some("abcd".into());
        assert!(args.signed(&maya.keypair).holds().is_err(), "not a key");
        let (rules_one, _) = settings("p1", &maya, "maya", Kind::Profile);
        assert!(rules_one.notes_key.is_none() && rules_one.holds().is_ok(), "rules 1 are as they were");
    }

    /// A profile made under rules 2 before profiles named a notes key still
    /// holds, replays and takes what visitors leave plainly, as it did.
    #[test]
    fn a_rules_two_profile_without_a_notes_key_replays_and_stays_plain() {
        let (mut maya, mut ren) = (Person::new("maya's mac"), Person::new("ren's laptop"));
        let notes = envelope::OpenKey::from_seed(b"maya's words");
        let host = host_for(&maya);
        let (sealed, _) = sealed_profile(&maya, &notes);
        let mut args = sealed.args.clone();
        args.notes_key = None;
        let args = args.signed(&maya.keypair);
        assert!(args.holds().is_ok(), "its settings hold");
        let room_key = Keypair::from_seed("profile v2");
        let mut profile = Room::new(args, room_key.clone()).unwrap();
        assert!(!profile.seals_notes(), "and it doesn't seal");
        maya.call(&mut profile, &host, "admit", json!({ "account": one_device(&ren, "ren's root"), "name": "ren" })).unwrap();
        ren.call(&mut profile, &host, "leave_note", json!({ "body": "love the lamp" })).unwrap();
        ren.call(&mut profile, &host, "hire", json!({ "agent": "site-fixes", "what": "check my links", "pledge": "a coffee" })).unwrap();
        let again = Room::from_record(profile.record(), Some(room_key)).expect("its record replays");
        assert!(Room::check(&profile.record()).is_ok(), "and checks out as a copy");
        let moves = feed(&again);
        assert_eq!(moves.iter().find(|m| m["kind"] == "note").unwrap()["body"], "love the lamp", "the note reads plainly, as before");
        assert_eq!(moves.iter().find(|m| m["kind"] == "hire").unwrap()["title"], "check my links");
        let ren_reads = envelope::OpenKey::for_author(&ren.keypair, profile.id());
        let to_nobody = [ren_reads.public()];
        assert!(sealed_call(&mut ren, &mut profile, &host, "leave_note", json!({ "body": "sealed?" }), &to_nobody).is_err(), "a room that doesn't seal takes no envelopes");
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

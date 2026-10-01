//! The attacks a review of draft three proved against this crate, kept as
//! tests: each one must be refused.

use chrono::{TimeDelta, Utc};
use diverge_desktop_room::account::{Proof, device_list};
use diverge_desktop_room::room::FEED;
use diverge_desktop_room::seal::{account_room_id, canonical, digest};
use diverge_desktop_room::*;
use rmcp::model::{CallToolRequestParams, RequestMetaObject, ResourceContents};
use serde_json::{Value, json};

struct H(Keypair);

impl Host for H {
    fn seal(&self, kind: &str, body: Value) -> Result<Statement, String> {
        Ok(Statement::make(&self.0, kind, body))
    }
}

struct P {
    k: Keypair,
    c: u64,
}

impl P {
    fn new(seed: &str) -> Self {
        P { k: Keypair::from_seed(seed), c: 0 }
    }
    fn call(&mut self, r: &mut Room, h: &H, verb: &'static str, a: Value) -> Result<(), String> {
        self.c += 1;
        let mut p = CallToolRequestParams::new(verb).with_arguments(a.as_object().cloned().unwrap());
        seal_call(&self.k, r.id(), &mut p, self.c);
        r.call(p, h).map(|_| ()).map_err(|e| e.message.to_string())
    }
}

/// What a forger does after editing a copy: recompute every hash and link.
/// They can't recompute the room's countersigns.
fn relink(moves: &mut [Move]) {
    let mut prev = String::new();
    for m in moves.iter_mut() {
        m.prev = prev.clone();
        let mut v = serde_json::to_value(&*m).unwrap();
        v.as_object_mut().unwrap().remove("hash");
        v.as_object_mut().unwrap().remove("room_sig");
        m.hash = digest(canonical(&v).as_bytes());
        prev = m.hash.clone();
    }
}

fn settings(label: &str, host: &P, name: &str) -> (Args, Keypair) {
    let room_key = Keypair::from_seed(&format!("room {label} {name}"));
    let args = Args {
        id: room_id(label, &host.k.key()),
        title: "W".into(),
        kind: Kind::Board,
        host_key: host.k.key(),
        host_name: name.into(),
        charter: "#".into(),
        open_door: false,
        continues: None,
        room_key: room_key.key(),
        at: Utc::now() - TimeDelta::days(1),
        rules: 1,
        host_account: None,
        sig: String::new(),
    }
    .signed(&host.k);
    (args, room_key)
}

fn feed(r: &Room) -> Vec<Value> {
    let x = r.read(FEED).unwrap();
    let ResourceContents::TextResourceContents { text, .. } = &x.contents[0] else { panic!() };
    serde_json::from_str(text).unwrap()
}

/// maya's workshop: ren let in, a show, a task done with a receipt, then ren removed.
fn workshop() -> (Room, P, P, H) {
    let (mut maya, mut ren) = (P::new("maya"), P::new("ren"));
    let h = H(maya.k.clone());
    let (args, key) = settings("workshop", &maya, "maya");
    let mut room = Room::new(args, key).unwrap();
    maya.call(&mut room, &h, "admit", json!({ "key": ren.k.key(), "name": "ren" })).unwrap();
    maya.call(&mut room, &h, "show", json!({ "title": "Draft" })).unwrap();
    maya.call(&mut room, &h, "post_task", json!({ "title": "T", "spec": "S" })).unwrap();
    ren.call(&mut room, &h, "claim", json!({ "task_id": "task-3" })).unwrap();
    ren.call(&mut room, &h, "deliver", json!({ "task_id": "task-3", "summary": "x" })).unwrap();
    maya.call(&mut room, &h, "accept", json!({ "task_id": "task-3" })).unwrap();
    maya.call(&mut room, &h, "remove", json!({ "key": ren.k.key() })).unwrap();
    (room, maya, ren, h)
}

#[test]
fn a_copy_with_a_rewritten_move_is_refused() {
    let (room, ..) = workshop();
    let mut rec = room.record();
    rec.moves[1].title = "I quit, ren runs this now".into();
    relink(&mut rec.moves);
    assert!(Room::check(&rec).is_err());
}

#[test]
fn a_copy_with_a_move_cut_from_the_middle_is_refused() {
    let (room, ..) = workshop();
    let mut rec = room.record();
    rec.moves.retain(|m| m.kind != "show");
    relink(&mut rec.moves);
    assert!(Room::check(&rec).is_err());
}

#[test]
fn a_copy_cut_short_is_a_true_copy_of_an_earlier_moment() {
    // What no record can show is what came after it. A copy ending before
    // ren's removal checks; it's what the room was then, and says when it ends.
    let (room, ..) = workshop();
    let mut rec = room.record();
    rec.moves.pop();
    let copy = Room::check(&rec).unwrap();
    assert!(copy.moves().last().unwrap().at <= room.moves().last().unwrap().at);
}

#[test]
fn a_successor_built_on_a_rewritten_history_is_refused() {
    let (room, _, ren, _) = workshop();
    let mut forged = room.record();
    forged.moves.retain(|m| m.kind != "removed");
    forged.moves[1].title = "ren runs this now".into();
    relink(&mut forged.moves);
    let (mut args, key) = settings("next", &ren, "ren");
    args.continues = Some(Continues { room: room.id().into(), title: "W".into(), last: forged.moves.last().unwrap().hash.clone() });
    let args = args.signed(&ren.k);
    assert!(Room::from_record(Record { args, before: Some(Box::new(forged)), moves: Vec::new() }, Some(key)).is_err());
}

#[test]
fn a_removed_member_cannot_continue_the_room() {
    let (room, _, ren, _) = workshop();
    let (mut args, key) = settings("next", &ren, "ren");
    args.continues = Some(Continues { room: room.id().into(), title: "W".into(), last: room.last_hash() });
    let args = args.signed(&ren.k);
    let err = Room::from_record(Record { args, before: Some(Box::new(room.record())), moves: Vec::new() }, Some(key)).err().unwrap();
    assert!(err.contains("still in"), "{err}");
}

#[test]
fn a_receipt_spliced_into_a_copy_is_refused() {
    let (room, ..) = workshop();
    let mallory = Keypair::from_seed("mallory");
    let fake = Statement::make(&mallory, "receipt", json!({ "title": "T", "to_person": mallory.key(), "host": "maya" }));
    let mut rec = room.record();
    let i = rec.moves.iter().position(|m| m.kind == "receipt").unwrap();
    rec.moves[i].fields.insert("statement".into(), serde_json::to_value(&fake).unwrap());
    relink(&mut rec.moves);
    assert!(Room::check(&rec).is_err());
}

#[test]
fn a_continued_room_restarts() {
    let (mut maya, mut ren) = (P::new("maya"), P::new("ren"));
    let h = H(maya.k.clone());
    let (args, key) = settings("old", &maya, "maya");
    let mut old = Room::new(args, key).unwrap();
    maya.call(&mut old, &h, "admit", json!({ "key": ren.k.key(), "name": "ren" })).unwrap();
    let (mut args, key) = settings("next", &ren, "ren");
    args.continues = Some(Continues { room: old.id().into(), title: "W".into(), last: old.last_hash() });
    let args = args.signed(&ren.k);
    let mut next = Room::from_record(Record { args, before: Some(Box::new(old.record())), moves: Vec::new() }, Some(key.clone())).unwrap();
    ren.c = 0;
    ren.call(&mut next, &H(ren.k.clone()), "show", json!({ "title": "still here" })).unwrap();
    let again = Room::from_record(next.record(), Some(key)).unwrap();
    assert_eq!(feed(&again), feed(&next));
}

#[test]
fn a_finished_task_gives_one_receipt() {
    let (mut room, mut maya, mut ren, h) = workshop();
    // ren was removed in the fixture; let them back in to try.
    maya.call(&mut room, &h, "admit", json!({ "key": ren.k.key(), "name": "ren" })).unwrap();
    assert!(ren.call(&mut room, &h, "deliver", json!({ "task_id": "task-3", "summary": "again" })).is_err());
    assert!(maya.call(&mut room, &h, "accept", json!({ "task_id": "task-3" })).is_err());
    assert_eq!(feed(&room).iter().filter(|m| m["kind"] == "receipt").count(), 1);
}

#[test]
fn a_seal_from_one_room_is_refused_in_another() {
    let (room, maya, mut ren, _) = workshop();
    // ren can't open a room by maya's id…
    let (mut taken, key) = settings("workshop", &ren, "ren");
    taken.id = room.id().into();
    assert!(Room::new(taken.signed(&ren.k), key).is_err());
    // …and maya's sealed show, replayed into ren's own room, is refused.
    let (args, key) = settings("workshop", &ren, "ren");
    let mut other = Room::new(args, key).unwrap();
    let ren_host = H(ren.k.clone());
    ren.c = 100;
    ren.call(&mut other, &ren_host, "admit", json!({ "key": maya.k.key(), "name": "maya" })).unwrap();
    let show = room.moves().iter().find(|m| m.kind == "show").unwrap().clone();
    let mut p = CallToolRequestParams::new("show").with_arguments(show.args.clone());
    let mut meta = serde_json::Map::new();
    meta.insert("network.diverge.desktop/seal".into(), serde_json::to_value(&show.seal).unwrap());
    p.meta = Some(RequestMetaObject::from(meta));
    assert!(other.call(p, &ren_host).is_err());
}

#[test]
fn a_host_only_move_by_a_member_in_a_copy_is_refused() {
    let (room, _, mut ren, _) = workshop();
    let mallory = Keypair::from_seed("mallory");
    // ren makes a well-formed admit in a room of their own, then splices it in.
    let (args, key) = settings("tmp", &ren, "ren");
    let mut tmp = Room::new(args, key).unwrap();
    ren.call(&mut tmp, &H(ren.k.clone()), "admit", json!({ "key": mallory.key(), "name": "mallory" })).unwrap();
    let mut spliced = tmp.moves()[0].clone();
    // Sealed by ren for maya's room itself, so the seal holds; the room must still refuse it.
    let mut p = CallToolRequestParams::new("admit").with_arguments(spliced.args.clone());
    seal_call(&ren.k, room.id(), &mut p, 50);
    spliced.seal = diverge_desktop_room::seal::seal_of(&p).unwrap();
    let mut rec = room.record();
    rec.moves.truncate(2);
    spliced.seq = 3;
    spliced.id = "admitted-3".into();
    rec.moves.push(spliced);
    relink(&mut rec.moves);
    let err = Room::check(&rec).err().unwrap();
    assert!(err.contains("only the host"), "{err}");
}

// --- rules versions, and rules 2: members are accounts -----------------------


/// Someone with an account: its root (kept here only to sign newer lists), and a device key per Mac.
struct A {
    root: Keypair,
    proof: Proof,
}

impl A {
    fn new(seed: &str, devices: &[&P]) -> Self {
        let root = Keypair::from_seed(&format!("{seed}'s root"));
        let proof = Proof::first(&root, Utc::now() - TimeDelta::days(2), &devices.iter().map(|d| d.k.key()).collect::<Vec<_>>());
        A { root, proof }
    }
    fn id(&self) -> String {
        self.proof.id()
    }
    /// The root's list number `n`, naming these devices.
    fn list(&self, n: u64, devices: &[&P]) -> Proof {
        Proof { genesis: self.proof.genesis.clone(), devices: device_list(&self.root, &self.id(), n, &devices.iter().map(|d| d.k.key()).collect::<Vec<_>>()) }
    }
}

/// A rules-2 room hosted by `host`'s account, signed on `device`.
fn settings_v2(label: &str, host: &A, device: &P, name: &str) -> (Args, Keypair) {
    let room_key = Keypair::from_seed(&format!("room {label} {name} v2"));
    let args = Args {
        id: account_room_id(label, &host.id()),
        title: "W".into(),
        kind: Kind::Board,
        host_key: device.k.key(),
        host_name: name.into(),
        charter: "#".into(),
        open_door: false,
        continues: None,
        room_key: room_key.key(),
        at: Utc::now() - TimeDelta::days(1),
        rules: 2,
        host_account: Some(host.proof.clone()),
        sig: String::new(),
    }
    .signed(&device.k);
    (args, room_key)
}

#[test]
fn a_record_from_before_rules_had_numbers_replays_and_its_signature_holds() {
    let (room, ..) = workshop();
    let json = serde_json::to_value(room.record()).unwrap();
    assert!(json["args"].get("rules").is_none(), "a version-1 room's settings don't say rules");
    assert!(json["args"].get("host_account").is_none());
    let back: Record = serde_json::from_value(json).unwrap();
    assert_eq!(back.args.rules, 1, "no rules number reads as the first rules");
    back.args.holds().unwrap();
    let again = Room::check(&back).unwrap();
    assert_eq!(feed(&again), feed(&room));
}

#[test]
fn version_one_settings_sign_the_same_bytes_as_before_there_were_versions() {
    let maya = P::new("maya");
    let (args, _) = settings("workshop", &maya, "maya");
    // Exactly the fields a room's settings had before rules were numbered.
    let before = json!({
        "id": args.id, "title": args.title, "kind": "board", "host_key": args.host_key, "host_name": args.host_name,
        "charter": args.charter, "open_door": false, "room_key": args.room_key, "at": args.at,
    });
    assert_eq!(canonical(&args.body()), canonical(&before));
    let text = serde_json::to_string(&args).unwrap();
    assert!(!text.contains("rules") && !text.contains("host_account"), "{text}");
}

#[test]
fn a_rules_number_this_program_doesnt_know_is_refused_in_words() {
    let maya = P::new("maya");
    let (mut args, key) = settings("workshop", &maya, "maya");
    args.rules = 99;
    let args = args.signed(&maya.k);
    let err = Room::new(args.clone(), key).err().unwrap();
    assert!(err.contains("99") && err.contains("rules"), "{err}");
    let err = Room::check(&Record { args, before: None, moves: Vec::new() }).err().unwrap();
    assert!(err.contains("99"), "{err}");
}

/// maya hosts a rules-2 room from her Mac; ren's account is let in.
fn workshop_v2() -> (Room, P, A, P, A, H) {
    let (maya_mac, ren_mac) = (P::new("maya's mac"), P::new("ren's laptop"));
    let (maya, ren) = (A::new("maya", &[&maya_mac]), A::new("ren", &[&ren_mac]));
    let h = H(maya_mac.k.clone());
    let (args, key) = settings_v2("workshop", &maya, &maya_mac, "maya");
    let mut room = Room::new(args, key).unwrap();
    let mut m = maya_mac;
    m.call(&mut room, &h, "admit", json!({ "account": ren.proof, "name": "ren" })).unwrap();
    (room, m, maya, ren_mac, ren, h)
}

#[test]
fn under_rules_two_a_member_is_their_account() {
    let (mut room, mut maya_mac, maya, mut ren_mac, ren, h) = workshop_v2();
    assert_eq!(room.id(), account_room_id("workshop", &maya.id()));
    assert!(room.member(&ren.id()).is_some(), "keyed by the account");
    assert!(room.member(&ren_mac.k.key()).is_none(), "not by the key");
    ren_mac.call(&mut room, &h, "show", json!({ "title": "a radio" })).unwrap();
    let shown = room.moves().last().unwrap().clone();
    assert_eq!((shown.by.as_str(), shown.actor()), (ren_mac.k.key().as_str(), ren.id().as_str()), "the move names the key that sealed it and the member it acts for");
    // A person is let in by their account, never a bare key.
    let ada = P::new("ada");
    assert!(maya_mac.call(&mut room, &h, "admit", json!({ "key": ada.k.key(), "name": "ada" })).is_err());
    assert!(Room::check(&room.record()).is_ok());
}

#[test]
fn a_v2_admit_with_a_forged_device_list_is_refused() {
    let (mut room, mut maya_mac, _, _, _, h) = workshop_v2();
    let (ada_mac, mallory) = (P::new("ada's mac"), P::new("mallory"));
    let ada = A::new("ada", &[&ada_mac]);
    // mallory writes a list adding her own key to ada's account, signed by herself.
    let forged = Proof { genesis: ada.proof.genesis.clone(), devices: device_list(&mallory.k, &ada.id(), 2, &[ada_mac.k.key(), mallory.k.key()]) };
    let err = maya_mac.call(&mut room, &h, "admit", json!({ "account": forged, "name": "ada" })).unwrap_err();
    assert!(err.contains("root"), "{err}");
    // A list ada's root signed for another account doesn't carry over.
    let elsewhere = Proof { genesis: ada.proof.genesis.clone(), devices: device_list(&ada.root, "another account", 2, &[mallory.k.key()]) };
    assert!(maya_mac.call(&mut room, &h, "admit", json!({ "account": elsewhere, "name": "ada" })).is_err());
    maya_mac.call(&mut room, &h, "admit", json!({ "account": ada.proof, "name": "ada" })).unwrap();
    // Nor can mallory bring a forged newer list into the room herself.
    let mut m = mallory;
    let err = m.call(&mut room, &h, "keys", json!({ "account": forged })).unwrap_err();
    assert!(err.contains("root"), "{err}");
}

#[test]
fn a_device_list_no_newer_than_the_one_held_is_refused() {
    let (mut room, _, _, mut ren_mac, ren, h) = workshop_v2();
    let second = P::new("ren's desktop");
    ren_mac.call(&mut room, &h, "keys", json!({ "account": ren.list(2, &[&ren_mac, &second]) })).unwrap();
    let err = ren_mac.call(&mut room, &h, "keys", json!({ "account": ren.list(1, &[&ren_mac]) })).unwrap_err();
    assert!(err.contains("number 1") && err.contains("number 2"), "{err}");
    assert!(ren_mac.call(&mut room, &h, "keys", json!({ "account": ren.list(2, &[&ren_mac]) })).is_err(), "the same number, saying something else");
    assert!(Room::check(&room.record()).is_ok());
}

#[test]
fn letting_someone_back_in_with_an_older_device_list_is_refused() {
    let (mut room, mut maya_mac, _, mut ren_mac, ren, h) = workshop_v2();
    let desktop = P::new("ren's desktop");
    // ren drops the laptop; then maya removes ren.
    ren_mac.call(&mut room, &h, "keys", json!({ "account": ren.list(2, &[&desktop]) })).unwrap();
    maya_mac.call(&mut room, &h, "remove", json!({ "key": ren.id() })).unwrap();
    // Letting ren back in with the list from before would bring the dropped laptop back.
    let err = maya_mac.call(&mut room, &h, "admit", json!({ "account": ren.proof, "name": "ren" })).unwrap_err();
    assert!(err.contains("number 1") && err.contains("number 2"), "{err}");
    assert!(maya_mac.call(&mut room, &h, "admit", json!({ "account": ren.list(2, &[&ren_mac]), "name": "ren" })).is_err(), "nor the same number saying something else");
    maya_mac.call(&mut room, &h, "admit", json!({ "account": ren.list(2, &[&desktop]), "name": "ren" })).unwrap();
    assert!(ren_mac.call(&mut room, &h, "show", json!({ "title": "x" })).is_err(), "the laptop stays off");
    assert!(Room::check(&room.record()).is_ok());
}

#[test]
fn a_device_taken_off_the_list_is_refused_from_then_on_and_its_earlier_moves_still_replay() {
    let (mut room, _, _, mut ren_mac, ren, h) = workshop_v2();
    let mut desktop = P::new("ren's desktop");
    ren_mac.call(&mut room, &h, "show", json!({ "title": "from the laptop" })).unwrap();
    // The laptop is gone: the root's list number 2 names only the desktop, which brings it.
    desktop.call(&mut room, &h, "keys", json!({ "account": ren.list(2, &[&desktop]) })).unwrap();
    let err = ren_mac.call(&mut room, &h, "show", json!({ "title": "from the lost laptop" })).unwrap_err();
    assert!(err.contains("no longer acts for ren"), "{err}");
    desktop.call(&mut room, &h, "show", json!({ "title": "from the desktop" })).unwrap();
    let copy = Room::check(&room.record()).unwrap();
    let titles: Vec<Value> = feed(&copy).iter().filter(|m| m["kind"] == "show").map(|m| m["title"].clone()).collect();
    assert_eq!(titles, [json!("from the laptop"), json!("from the desktop")], "the laptop's move still replays; its later one never landed");
    // A copy where the laptop's late move is spliced in after the newer list is refused.
    let mut late = P::new("x");
    late.k = Keypair::from_seed("ren's laptop");
    late.c = 99;
    let (args, key) = settings_v2("scratch", &A::new("scratch", &[&late]), &late, "x");
    let mut scratch = Room::new(args, key).unwrap();
    late.call(&mut scratch, &H(late.k.clone()), "show", json!({ "title": "spliced" })).unwrap();
    let mut spliced = scratch.moves()[0].clone();
    let mut p = CallToolRequestParams::new("show").with_arguments(spliced.args.clone());
    seal_call(&late.k, room.id(), &mut p, 100);
    spliced.seal = diverge_desktop_room::seal::seal_of(&p).unwrap();
    spliced.member = Some(ren.id());
    let mut rec = room.record();
    spliced.seq = rec.moves.len() as u64 + 1;
    spliced.id = format!("show-{}", spliced.seq);
    rec.moves.push(spliced);
    relink(&mut rec.moves);
    let err = Room::check(&rec).err().unwrap();
    assert!(err.contains("would have been refused"), "{err}");
}

#[test]
fn a_host_on_a_second_device_of_the_same_account_hosts_the_same_room() {
    let (room, _, maya, _, ren, h) = workshop_v2();
    let id = room.id().to_owned();
    let mut desk = P::new("maya's desk");
    let mut room = room;
    // The host's root names a second device; that device brings the list and acts as host.
    desk.call(&mut room, &h, "keys", json!({ "account": maya.list(2, &[&P::new("maya's mac"), &desk]) })).unwrap();
    let ada_mac = P::new("ada's mac");
    let ada = A::new("ada", &[&ada_mac]);
    desk.call(&mut room, &h, "admit", json!({ "account": ada.proof, "name": "ada" })).unwrap();
    desk.call(&mut room, &h, "remove", json!({ "key": ren.id() })).unwrap();
    // The room's id names the account, so the same id from either device.
    assert_eq!(account_room_id("workshop", &maya.list(2, &[&desk]).id()), id);
    // Restarted from its record, it's the same room, and the desk still hosts it.
    let key = Keypair::from_seed("room workshop maya v2");
    let mut again = Room::from_record(room.record(), Some(key)).unwrap();
    assert_eq!(again.id(), id);
    desk.call(&mut again, &h, "set_charter", json!({ "text": "# from the desk" })).unwrap();
    // Someone else's account can't host by that id, whatever device signs.
    let (mallory_mac, mallory) = (P::new("mallory"), A::new("mallory", &[&P::new("mallory")]));
    let (mut taken, k) = settings_v2("workshop", &mallory, &mallory_mac, "mallory");
    taken.id = id;
    assert!(Room::new(taken.signed(&mallory_mac.k), k).is_err());
}

#[test]
fn under_rules_two_someone_unlisted_is_a_mark_of_their_account_until_they_bring_their_list() {
    let (mut room, mut maya_mac, _, _, _, h) = workshop_v2();
    let mut ada_mac = P::new("ada's mac");
    let ada = A::new("ada", &[&ada_mac]);
    let rid = room.id().to_owned();
    maya_mac.call(&mut room, &h, "admit", json!({ "key_mark": key_mark(&rid, &ada.id()), "name": "ada", "listed": false })).unwrap();
    let record = serde_json::to_string(&room.record()).unwrap();
    assert!(!record.contains(&ada.id()) && !record.contains(&ada_mac.k.key()), "the record names neither her account nor her key");
    assert!(room.may_read(&ada.id()));
    assert!(ada_mac.call(&mut room, &h, "show", json!({ "title": "x" })).unwrap_err().contains("not a member"), "the room doesn't know her key yet");
    ada_mac.call(&mut room, &h, "keys", json!({ "account": ada.proof })).unwrap();
    ada_mac.call(&mut room, &h, "show", json!({ "title": "a radio" })).unwrap();
    assert_eq!(feed(&room).iter().find(|m| m["kind"] == "show").unwrap()["author"], "ada");
    assert!(room.member(&ada.id()).is_some());
    assert!(Room::check(&room.record()).is_ok());
}

#[test]
fn a_receipt_from_a_rules_two_room_proves_its_room_wherever_it_is_pinned() {
    let (mut room, mut maya_mac, maya, mut ren_mac, ren, h) = workshop_v2();
    maya_mac.call(&mut room, &h, "post_task", json!({ "title": "T", "spec": "S" })).unwrap();
    let task = room.moves().last().unwrap().id.clone();
    ren_mac.call(&mut room, &h, "claim", json!({ "task_id": task })).unwrap();
    ren_mac.call(&mut room, &h, "deliver", json!({ "task_id": task, "summary": "x" })).unwrap();
    maya_mac.call(&mut room, &h, "accept", json!({ "task_id": task })).unwrap();
    let receipt = room.moves().last().unwrap().clone();
    let statement: Statement = serde_json::from_value(receipt.fields["statement"].clone()).unwrap();
    assert_eq!(receipt_issuer(&statement), Some(maya.id()), "the room's id names the account the receipt carries");
    assert_eq!(statement.field("to_person"), Some(ren.id().as_str()));
    assert!(Room::check(&room.record()).is_ok());
    // Sealed by a key the account doesn't name, it proves nothing.
    let mallory = Keypair::from_seed("mallory");
    let forged = Statement::make(&mallory, "receipt", statement.body.clone());
    assert_eq!(receipt_issuer(&forged), None);
    // ren pins it on a profile ren's account hosts.
    let room_key = Keypair::from_seed("ren's profile");
    let args = Args {
        id: account_room_id("profile", &ren.id()),
        title: "ren".into(),
        kind: Kind::Profile,
        host_key: ren_mac.k.key(),
        host_name: "ren".into(),
        charter: "#".into(),
        open_door: true,
        continues: None,
        room_key: room_key.key(),
        at: Utc::now() - TimeDelta::days(1),
        rules: 2,
        host_account: Some(ren.proof.clone()),
        sig: String::new(),
    }
    .signed(&ren_mac.k);
    let mut profile = Room::new(args, room_key).unwrap();
    let ren_h = H(ren_mac.k.clone());
    assert!(ren_mac.call(&mut profile, &ren_h, "pin_receipt", json!({ "statement": forged })).is_err());
    ren_mac.call(&mut profile, &ren_h, "pin_receipt", json!({ "statement": statement })).unwrap();
}

/// Continue `old` under rules 2, from an account naming `devices`, signed on `signer`.
fn continue_as(old: &Room, label: &str, account: &A, signer: &P) -> Result<Room, String> {
    let (mut args, key) = settings_v2(label, account, signer, "eve");
    args.continues = Some(Continues { room: old.id().into(), title: "W".into(), last: old.last_hash() });
    let args = args.signed(&signer.k);
    Room::from_record(Record { args, before: Some(Box::new(old.record())), moves: Vec::new() }, Some(key))
}

#[test]
fn a_removed_member_whose_list_names_a_current_members_key_cannot_continue_a_rules_one_room() {
    // ren was removed; maya is still in. ren's root may name maya's key, but ren doesn't hold it.
    let (room, maya, ren, _) = workshop();
    let eve = A::new("ren's other account", &[&ren, &maya]);
    let err = continue_as(&room, "next", &eve, &ren).err().unwrap();
    assert!(err.contains("still in"), "{err}");
    // maya, signing herself, still can.
    let hers = A::new("maya", &[&maya]);
    continue_as(&room, "next", &hers, &maya).unwrap();
}

#[test]
fn a_removed_member_whose_list_names_a_current_members_key_cannot_continue_a_rules_two_room() {
    let (mut room, mut maya_mac, maya, ren_mac, ren, h) = workshop_v2();
    maya_mac.call(&mut room, &h, "remove", json!({ "key": ren.id() })).unwrap();
    let maya_key = P::new("maya's mac");
    // A fresh account naming ren's laptop and maya's Mac…
    let eve = A::new("eve", &[&ren_mac, &maya_key]);
    let err = continue_as(&room, "next", &eve, &ren_mac).err().unwrap();
    assert!(err.contains("still in"), "{err}");
    // …or ren's own account with a newer list that adds maya's Mac.
    let ren_next = A { root: Keypair::from_seed("ren's root"), proof: ren.list(2, &[&ren_mac, &maya_key]) };
    let err = continue_as(&room, "next", &ren_next, &ren_mac).err().unwrap();
    assert!(err.contains("still in"), "{err}");
    // maya, from her account, still can.
    continue_as(&room, "next", &maya, &maya_key).unwrap();
}

#[test]
fn a_key_named_on_one_members_list_cant_be_let_in_for_anyone_else() {
    // A device list is its root's word alone: nothing shows the keys on it agreed.
    // So a key one member lists, held or not, is refused for anyone else, its holder included.
    let (mut room, mut maya_mac, _, mut ren_mac, ren, h) = workshop_v2();
    let ada_mac = P::new("ada's mac");
    ren_mac.call(&mut room, &h, "keys", json!({ "account": ren.list(2, &[&ren_mac, &ada_mac]) })).unwrap();
    let ada = A::new("ada", &[&ada_mac]);
    let err = maya_mac.call(&mut room, &h, "admit", json!({ "account": ada.proof, "name": "ada" })).unwrap_err();
    assert!(err.contains("already acts for someone else"), "{err}");
    // It can't seal a move for ren without the key itself.
    let mut fake = P::new("mallory");
    fake.c = 50;
    assert!(fake.call(&mut room, &h, "show", json!({ "title": "x" })).is_err());
}

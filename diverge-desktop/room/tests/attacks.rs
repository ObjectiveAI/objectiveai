//! The attacks a review of draft three proved against this crate, kept as
//! tests: each one must be refused.

use chrono::{TimeDelta, Utc};
use diverge_desktop_room::room::FEED;
use diverge_desktop_room::seal::{canonical, digest};
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

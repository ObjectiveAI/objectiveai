//! Key marks: telling apart two people (or agents) in one room who go by
//! the same name.
//!
//! A name is whatever someone typed, so two members of a room can share
//! one. Where they do, each gets a short mark beside the name, drawn from
//! the room and their key together. The same key carries a different mark
//! in every room, so a mark can't be used to tie a fresh name in one room
//! to the same key in another. Marks are the app's, worked out from the
//! room's own member list; the room doesn't hand them out.

use std::collections::HashMap;

use crate::view::MemberView;

/// The fewest hex characters a mark has. Longer only when two members a
/// mark has to tell apart would otherwise get the same one.
pub const SHORTEST: usize = 6;

/// A key's mark in one room, `len` hex characters long.
pub fn mark(room: &str, key: &str, len: usize) -> String {
    let digest = diverge_desktop_room::seal::digest(format!("diverge-desktop name mark\n{room}\n{key}").as_bytes());
    digest[..len.clamp(1, digest.len())].to_owned()
}

/// Names that read the same: spacing and case don't tell people apart.
fn same(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// For each (name, key) in one room: a mark when another key there goes by
/// the same name, as short as tells them all apart; nothing otherwise.
pub fn shared_names(room: &str, who: &[(&str, &str)]) -> Vec<Option<String>> {
    let mut by_name: HashMap<String, Vec<&str>> = HashMap::new();
    for (name, key) in who {
        let keys = by_name.entry(same(name)).or_default();
        if !key.is_empty() && !keys.contains(key) {
            keys.push(key);
        }
    }
    let mut lengths: HashMap<String, usize> = HashMap::new();
    for (name, keys) in &by_name {
        if keys.len() < 2 {
            continue;
        }
        let mut len = SHORTEST;
        while len < 64 && {
            let mut marks: Vec<String> = keys.iter().map(|k| mark(room, k, len)).collect();
            marks.sort();
            marks.dedup();
            marks.len() < keys.len()
        } {
            len += 2;
        }
        lengths.insert(name.clone(), len);
    }
    who.iter().map(|(name, key)| lengths.get(&same(name)).filter(|_| !key.is_empty()).map(|len| mark(room, key, *len))).collect()
}

/// Mark a room's member list where two members share a name.
pub fn members(room: &str, list: &mut [MemberView]) {
    let who: Vec<(&str, &str)> = list.iter().map(|m| (m.name.as_str(), m.key.as_str())).collect();
    let marks = shared_names(room, &who);
    for (m, mark) in list.iter_mut().zip(marks) {
        m.mark = mark;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, key: &str) -> MemberView {
        serde_json::from_value(serde_json::json!({ "name": name, "key": key, "is_agent": false, "joined": "" })).unwrap()
    }

    #[test]
    fn two_members_of_a_room_with_the_same_name_show_different_marks() {
        let mut list = vec![member("ada", "a1"), member("Ada ", "b2"), member("ren", "c3")];
        members("room-1", &mut list);
        let (first, second) = (list[0].mark.clone().expect("marked"), list[1].mark.clone().expect("marked"));
        assert_ne!(first, second);
        assert!(first.len() >= SHORTEST);
        assert_eq!(list[2].mark, None, "a name nobody else has needs no mark");
        // Alone in a room, the same key carries no mark at all.
        let mut alone = vec![member("ada", "a1"), member("ren", "c3")];
        members("room-1", &mut alone);
        assert!(alone.iter().all(|m| m.mark.is_none()));
    }

    #[test]
    fn a_fresh_names_mark_differs_from_room_to_room() {
        let fresh = "f".repeat(64);
        let in_a = shared_names("room-a", &[("lamp person", &fresh), ("lamp person", "someone else")]);
        let in_b = shared_names("room-b", &[("lamp person", &fresh), ("lamp person", "another")]);
        assert_ne!(in_a[0], in_b[0], "the same key, not linked across rooms by its mark");
        assert_eq!(mark("room-a", &fresh, SHORTEST), in_a[0].clone().unwrap(), "drawn from the room and the key, nothing else");
    }

    #[test]
    fn marks_grow_until_they_tell_everyone_apart() {
        // Many people sharing one name: however many, no two marks are the same.
        let keys: Vec<String> = (0..5000).map(|i| format!("key {i}")).collect();
        let who: Vec<(&str, &str)> = keys.iter().map(|k| ("ada", k.as_str())).collect();
        let marks: Vec<String> = shared_names("room-1", &who).into_iter().map(Option::unwrap).collect();
        let mut distinct = marks.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), marks.len());
    }
}

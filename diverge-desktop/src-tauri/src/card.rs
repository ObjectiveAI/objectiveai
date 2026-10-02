//! Your card: a picture, a few words about you, links. Each part has who
//! sees it, and you set it: anyone with your profile's link (it rides on
//! the invite, so the door shows it before anyone knocks), the people you
//! let into your profile room, or only you (it stays in this folder and is
//! never sent). The room keeps only what it's sent; the door only what the
//! invite carries.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use ts_rs::TS;

/// Who sees one part of your card. You set it; the app's starting position is `You`.
#[derive(Serialize, Deserialize, TS, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum Who {
    /// Anyone with your profile's link: it's on your door.
    Link,
    /// The people you let into your profile room.
    Room,
    /// Only you: kept on this Mac, never sent.
    #[default]
    You,
}

#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct CardLink {
    pub title: String,
    pub url: String,
}

/// Your card as you keep it, every part with who sees it.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq, Default)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct Card {
    /// A data: address (PNG, JPEG or WebP), or empty.
    #[serde(default)]
    pub picture: String,
    #[serde(default)]
    pub picture_who: Who,
    #[serde(default)]
    pub about: String,
    #[serde(default)]
    pub about_who: Who,
    #[serde(default)]
    pub links: Vec<CardLink>,
    #[serde(default)]
    pub links_who: Who,
}

/// What a door shows before anyone knocks: the parts set to anyone with the link.
#[derive(Serialize, Deserialize, TS, Clone, Debug, PartialEq, Eq, Default)]
#[ts(export, export_to = "../../src/bindings/")]
pub struct DoorCard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<CardLink>,
}

impl Card {
    /// The parts someone sees: at the door (`Link`), or let in (`Room`).
    fn seen_at(&self, place: Who) -> Map<String, Value> {
        let sees = |who: Who| match place {
            Who::Link => who == Who::Link,
            Who::Room => who != Who::You,
            Who::You => true,
        };
        let mut o = Map::new();
        if sees(self.picture_who) && !self.picture.trim().is_empty() {
            o.insert("picture".into(), json!(self.picture.trim()));
        }
        if sees(self.about_who) && !self.about.trim().is_empty() {
            o.insert("about".into(), json!(self.about.trim()));
        }
        let links: Vec<CardLink> = self.links.iter().filter(|l| !l.title.trim().is_empty() || !l.url.trim().is_empty()).map(|l| CardLink { title: l.title.trim().into(), url: l.url.trim().into() }).collect();
        if sees(self.links_who) && !links.is_empty() {
            o.insert("links".into(), json!(links));
        }
        o
    }

    /// Whether every part is one a room would keep, whoever sees it.
    pub fn check(&self) -> Result<(), String> {
        diverge_desktop_room::check_card(&self.seen_at(Who::You))
    }

    /// What `set_card` sends your profile room, or nothing when no part is for the people you let in.
    pub fn for_room(&self) -> Option<Map<String, Value>> {
        Some(self.seen_at(Who::Room)).filter(|o| !o.is_empty())
    }

    /// What your profile's invite carries.
    pub fn for_door(&self) -> Option<DoorCard> {
        let o = self.seen_at(Who::Link);
        if o.is_empty() {
            return None;
        }
        serde_json::from_value(Value::Object(o)).ok()
    }

    /// A card read back from your profile room, where it holds one and this folder doesn't:
    /// every part it shows is for the people you let in, since the room can't say which were on your door.
    pub fn from_room(m: &crate::view::MoveView) -> Card {
        let links = m.fields.get("links").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
        Card {
            picture: m.fields.get("picture").and_then(Value::as_str).unwrap_or_default().to_owned(),
            picture_who: Who::Room,
            about: m.body.clone(),
            about_who: Who::Room,
            links,
            links_who: Who::Room,
        }
    }
}

/// A card move in your profile room that still holds its words.
pub fn live(m: &crate::view::MoveView) -> bool {
    m.kind == "card" && m.fields.get("erased").is_none()
}

/// Whether a card move in the room says exactly what `want` would send.
pub fn says(m: &crate::view::MoveView, want: &Map<String, Value>) -> bool {
    let got = |k: &str| m.fields.get(k).filter(|v| !v.is_null());
    m.body.trim() == want.get("about").and_then(Value::as_str).unwrap_or_default() && got("picture") == want.get("picture") && got("links") == want.get("links")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Card {
        Card {
            picture: "data:image/png;base64,iVBORw0KGgo=".into(),
            picture_who: Who::Link,
            about: "I fix lamps on Saturdays.".into(),
            about_who: Who::Room,
            links: vec![CardLink { title: "My site".into(), url: "https://example.org".into() }],
            links_who: Who::You,
        }
    }

    #[test]
    fn each_part_goes_only_where_you_said() {
        let c = card();
        let door = c.for_door().unwrap();
        assert_eq!((door.picture.is_some(), door.about, door.links.len()), (true, None, 0), "the door: only what anyone with the link sees");
        let room = c.for_room().unwrap();
        assert!(room.contains_key("picture") && room.contains_key("about") && !room.contains_key("links"), "the room: the door's parts and the room's, never yours alone");
        let yours = Card { picture_who: Who::You, about_who: Who::You, ..c.clone() };
        assert!(yours.for_door().is_none() && yours.for_room().is_none(), "only you: nothing is sent");
    }

    #[test]
    fn a_new_card_starts_with_only_you() {
        let c: Card = serde_json::from_value(json!({ "about": "hi" })).unwrap();
        assert_eq!((c.picture_who, c.about_who, c.links_who), (Who::You, Who::You, Who::You));
        assert!(c.for_room().is_none());
    }

    #[test]
    fn a_part_the_room_wouldnt_keep_is_refused_even_if_only_you_see_it() {
        let c = Card { links: vec![CardLink { title: "x".into(), url: "file:///etc".into() }], ..card() };
        assert!(c.check().is_err());
        assert!(card().check().is_ok());
    }
}

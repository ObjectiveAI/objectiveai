//! What visitors leave on a profile, sealed between them and its owner.
//!
//! A profile under rules 2 names its owner's notes key in its signed
//! settings, and takes a note, a hire, an answer's note and a hire's result
//! only sealed (see `diverge_desktop_room::envelope`). The room, and every
//! copy of its record, holds ciphertext for them. This app seals what you
//! send before your key signs it ([`prepare`]), and opens what it can when
//! it reads a feed ([`open_moves`]): your profile's with your notes key,
//! and anything you wrote, or that was sealed back to you, with your own
//! keys for that room. What it can't open stays sealed, and says so.

use rmcp::model::{JsonObject, ResourceContents};
use serde_json::{Value, json};

use diverge_desktop_room::envelope::{self, OpenKey, SEALED_VERBS};
use diverge_desktop_room::room::{ABOUT, FEED};

use crate::identity::{Actor, Identity};
use crate::spaces::{Id, Spaces};
use crate::view::MoveView;

async fn read_text(spaces: &dyn Spaces, room: &Id, uri: &str) -> Option<String> {
    spaces.read(room, uri).await.ok()?.contents.into_iter().find_map(|c| match c {
        ResourceContents::TextResourceContents { text, .. } => Some(text),
        _ => None,
    })
}

/// The notes key a room's settings name, if it's a profile that seals what visitors leave.
pub async fn notes_key_of(spaces: &dyn Spaces, room: &Id) -> Option<String> {
    let about: Value = serde_json::from_str(&read_text(spaces, room, ABOUT).await?).ok()?;
    about["notes_key"].as_str().map(str::to_owned)
}

/// Your notes key, when it's the one a room's settings name: that profile is yours.
fn yours(identity: &Identity, notes_key: &str) -> Option<std::sync::Arc<OpenKey>> {
    identity.notes_key().ok().filter(|k| k.public() == notes_key)
}

/// What a hire asked, opened with your notes key: what, of which agent,
/// for what pledge, and where its answer goes back to.
pub struct Asked {
    pub agent: String,
    pub what: String,
    pub pledge: Option<String>,
    pub reply_to: Option<String>,
}

/// Open a hire's sealed ask with your notes key. `by` is the key that sealed the call.
pub fn open_ask(identity: &Identity, room: &str, notes_key: &str, sealed: &Value, by: &str) -> Option<Asked> {
    let key = yours(identity, notes_key)?;
    let words = envelope::open(&envelope::shape(sealed).ok()?, &key, room, "hire", by)?;
    let text = |k: &str| words.get(k).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(str::to_owned);
    Some(Asked { agent: text("agent").unwrap_or_default(), what: text("what").unwrap_or_default(), pledge: text("pledge"), reply_to: text("reply_to") })
}

/// A sealed verb's arguments, as they go to a room that seals them: the
/// words sealed to the owner's notes key and, for a note or a hire, to
/// you (a hire also names where its answer goes back to); for an answer
/// or a result, to the owner and whoever asked. Anything else goes as it is.
pub async fn prepare(identity: &Identity, spaces: &dyn Spaces, room: &Id, actor: &Actor, verb: &str, args: Value) -> Result<Value, String> {
    let Some(object) = args.as_object().filter(|a| SEALED_VERBS.contains(&verb) && !a.contains_key("sealed")) else { return Ok(args) };
    let Some(notes_key) = notes_key_of(spaces, room).await else { return Ok(args) };
    let (author, mine) = identity.author_key(actor, &room.id)?;
    let mut words: JsonObject = object.clone();
    let readers = match verb {
        "leave_note" | "hire" => {
            if verb == "hire" {
                words.insert("reply_to".into(), json!(mine.public()));
            }
            vec![notes_key, mine.public()]
        }
        _ => {
            // An answer or a result: back to whoever asked, as their sealed ask says.
            let hire_id = object.get("hire_id").and_then(Value::as_str).unwrap_or_default();
            let feed: Vec<Value> = serde_json::from_str(&read_text(spaces, room, FEED).await.unwrap_or_default()).unwrap_or_default();
            let asked = feed.iter().find(|m| m["id"] == hire_id && m["kind"] == "hire");
            let back = asked.and_then(|m| open_ask(identity, &room.id, &notes_key, &m["fields"]["sealed"], m["by"].as_str().unwrap_or_default())).and_then(|a| a.reply_to);
            std::iter::once(notes_key).chain(back).collect()
        }
    };
    envelope::seal_args(&room.id, verb, &words, &author, &readers).map(Value::Object)
}

/// Open the sealed moves of a room's feed that you can: with your notes key
/// on your own profile, and with your keys for that room for anything you
/// wrote or that was sealed back to you. Each sealed move says it was
/// sealed (`fields.sealed`) and whether this app opened it (`fields.opened`);
/// the envelope itself never reaches the page.
pub fn open_moves(identity: &Identity, room: &str, notes_key: Option<&str>, moves: &mut [MoveView]) {
    if !moves.iter().any(|m| m.fields.get("sealed").is_some_and(Value::is_object)) {
        return;
    }
    let owner = notes_key.and_then(|k| yours(identity, k));
    let authors = identity.author_keys(room);
    let keys: Vec<&OpenKey> = owner.as_deref().into_iter().chain(authors.iter()).collect();
    for m in moves.iter_mut() {
        let Some(sealed) = m.fields.get("sealed").filter(|v| v.is_object()).cloned() else { continue };
        let words = envelope::verb_of_kind(&m.kind).zip(envelope::shape(&sealed).ok()).and_then(|(verb, e)| keys.iter().find_map(|k| envelope::open(&e, k, room, verb, &m.by)));
        let Some(fields) = m.fields.as_object_mut() else { continue };
        fields.insert("sealed".into(), json!(true));
        fields.insert("opened".into(), json!(words.is_some()));
        let Some(words) = words else { continue };
        let text = |k: &str| words.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        match m.kind.as_str() {
            "note" => m.body = text("body"),
            "hire" => {
                m.title = text("what");
                for k in ["agent", "pledge"] {
                    if let Some(v) = words.get(k).filter(|v| v.as_str().is_some_and(|s| !s.is_empty())) {
                        fields.insert(k.into(), v.clone());
                    }
                }
            }
            "hire_answer" => m.body = text("note"),
            "hire_delivery" => {
                let (summary, result) = (text("summary"), text("result"));
                m.body = if result.is_empty() { summary } else { format!("{summary}\n\n{result}") };
                if let Some(files) = words.get("files") {
                    fields.insert("files".into(), files.clone());
                }
            }
            _ => {}
        }
    }
    // An answer or a result names its hire: it reads under that hire's words, where they were opened.
    let titles: std::collections::HashMap<String, String> = moves.iter().filter(|m| m.kind == "hire" && !m.title.is_empty()).map(|m| (m.id.clone(), m.title.clone())).collect();
    for m in moves.iter_mut().filter(|m| (m.kind == "hire_answer" || m.kind == "hire_delivery") && m.title.is_empty()) {
        if let Some(t) = m.parent.as_ref().and_then(|p| titles.get(p)) {
            m.title = t.clone();
        }
    }
}

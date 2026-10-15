//! Your agents, live: one `agents::list` held open for the app's life.
//!
//! The daemon's list says every agent, `Listed`, then what changes, and
//! stays open. The app holds exactly one, here, and keeps what it says in
//! one map; when the list ends or fails it is asked again a second later.
//! Every change makes a whole new snapshot with a higher generation, sent
//! to whatever watches (`agents_watch`), so nothing that watches ever
//! reconciles a delta: the pattern of [`crate::tabs`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use indexmap::IndexMap;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use diverge_sdk::daemon::endpoints::agents;
use diverge_sdk::daemon::endpoints::agents::list::server::response::Frame;
use diverge_sdk::daemon::endpoints::agents::templates::Template as AgentTemplate;

use crate::daemon::{Daemon, Frames};
use crate::view::{AgentView, AgentsSnapshot, ListState, agent_view, error_text};

/// How long the app waits before asking for the list again once it ended.
pub const AGAIN: Duration = Duration::from_secs(1);

/// What the live list needs of the daemon: its agents list, and what a
/// template (by id) says an agent runs.
#[async_trait]
pub trait Source: Send + Sync {
    fn list(&self, cancel: CancellationToken) -> Frames<Frame>;
    async fn template(&self, id: &str) -> Option<AgentTemplate>;
}

#[async_trait]
impl<D: Daemon + ?Sized> Source for D {
    fn list(&self, cancel: CancellationToken) -> Frames<Frame> {
        self.agents_list(agents::list::client::request::Frame { filter: Default::default(), count: None, relay: None }, cancel)
    }

    async fn template(&self, id: &str) -> Option<AgentTemplate> {
        use agents::templates::get::server::response::Frame as Got;
        match self.agents_templates_get(agents::templates::get::client::request::Frame { id: id.to_owned(), relay: None }).await {
            Got::Found(found) => Some(found.template),
            Got::NotFound | Got::Forbidden | Got::Error(_) => None,
        }
    }
}

pub struct Fleet {
    state: Mutex<State>,
    tx: watch::Sender<AgentsSnapshot>,
}

#[derive(Default)]
struct State {
    generation: u64,
    list: ListState,
    /// Every agent the list has said, by what names it for good, its
    /// template and index: a rename is a change to one agent, not another.
    agents: IndexMap<String, AgentView>,
    /// A list asked (again): what it has said so far, until it says it's whole.
    asked: Option<IndexMap<String, AgentView>>,
}

/// What names a list item for good.
fn id_of(agent: &agents::list::server::response::Agent) -> String {
    format!("{}#{}", agent.template, agent.index)
}

impl Fleet {
    pub fn new() -> Arc<Fleet> {
        Arc::new(Fleet { state: Mutex::new(State::default()), tx: watch::channel(AgentsSnapshot::default()).0 })
    }

    /// Ask the daemon, and follow what it says for the app's life.
    pub fn start(self: &Arc<Self>, daemon: Arc<dyn Daemon>) {
        let fleet = self.clone();
        tokio::spawn(async move { fleet.follow(daemon.as_ref(), CancellationToken::new()).await });
    }

    /// What the app knows now.
    pub fn snapshot(&self) -> AgentsSnapshot {
        self.tx.borrow().clone()
    }

    /// Every snapshot from now on, starting with this one.
    pub fn subscribe(&self) -> watch::Receiver<AgentsSnapshot> {
        self.tx.subscribe()
    }

    /// Hold the list open until `stop`; when it ends or fails, ask again a
    /// second later. A template's image is asked for once.
    pub async fn follow<S: Source + ?Sized>(&self, source: &S, stop: CancellationToken) {
        let mut templates: HashMap<String, Option<AgentTemplate>> = HashMap::new();
        loop {
            self.asked();
            let cancel = stop.child_token();
            let mut frames = source.list(cancel.clone());
            loop {
                let frame = tokio::select! {
                    _ = stop.cancelled() => return,
                    frame = frames.next() => frame,
                };
                let Some(frame) = frame else { break };
                let ends = match &frame {
                    Frame::Added(agent) | Frame::Changed(agent) => {
                        if !templates.contains_key(&agent.template) {
                            templates.insert(agent.template.clone(), source.template(&agent.template).await);
                        }
                        false
                    }
                    Frame::Removed(_) | Frame::Listed => false,
                    Frame::Forbidden | Frame::Error(_) => true,
                };
                self.apply(frame, &templates);
                if ends {
                    break;
                }
            }
            cancel.cancel();
            self.ended();
            tokio::select! {
                _ = stop.cancelled() => return,
                _ = tokio::time::sleep(AGAIN) => {}
            }
        }
    }

    /// A list was asked for: what it says is gathered until it's whole. A
    /// list that was live is waiting again; a refusal or a failure stands
    /// until the new list says otherwise.
    fn asked(&self) {
        let mut s = self.state.lock().unwrap();
        s.asked = Some(IndexMap::new());
        if s.list == ListState::Live {
            s.list = ListState::Waiting;
        }
        self.publish(&mut s);
    }

    /// The list finished without a word: it is asked again.
    fn ended(&self) {
        let mut s = self.state.lock().unwrap();
        s.asked = None;
        if s.list == ListState::Live {
            s.list = ListState::Waiting;
        }
        self.publish(&mut s);
    }

    /// One frame of the list, applied. A removal of an agent the app never
    /// heard of changes nothing.
    pub fn apply(&self, frame: Frame, templates: &HashMap<String, Option<AgentTemplate>>) {
        let mut guard = self.state.lock().unwrap();
        let s = &mut *guard;
        let gathering = match s.asked.as_mut() {
            Some(asked) => asked,
            None => &mut s.agents,
        };
        match frame {
            Frame::Added(agent) | Frame::Changed(agent) => {
                let template = templates.get(&agent.template).and_then(Option::as_ref);
                gathering.insert(id_of(&agent), agent_view(agent, template));
            }
            Frame::Removed(agent) => {
                gathering.shift_remove(&id_of(&agent));
            }
            Frame::Listed => {
                if let Some(whole) = s.asked.take() {
                    s.agents = whole;
                }
                s.list = ListState::Live;
            }
            // The daemon won't say which agents are yours: none are shown.
            Frame::Forbidden => {
                s.asked = None;
                s.agents.clear();
                s.list = ListState::Forbidden;
            }
            // What was listed before stays, said to be as of then.
            Frame::Error(error) => {
                s.asked = None;
                s.list = ListState::Error { message: error_text(&error) };
            }
        }
        self.publish(s);
    }

    /// A new snapshot, if anything in it changed.
    fn publish(&self, s: &mut State) {
        let agents: Vec<AgentView> = s.agents.values().cloned().collect();
        let same = {
            let last = self.tx.borrow();
            last.state == s.list && last.agents == agents
        };
        if same {
            return;
        }
        s.generation += 1;
        self.tx.send_replace(AgentsSnapshot { generation: s.generation, state: s.list.clone(), agents });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use diverge_sdk::daemon::creator::{self, Creator};
    use diverge_sdk::shared::error::Error as WireError;
    use serde_json::json;

    fn item(template: &str, index: u64, name: Option<&str>) -> agents::list::server::response::Agent {
        agents::list::server::response::Agent {
            name: name.map(Into::into),
            template: template.into(),
            index,
            creator: Creator::Client(creator::Client { identity: "you".into() }),
            created: Utc::now(),
            active: false,
            last_active: None,
            provider: None,
            logs_index: 0,
            tools: Vec::new(),
            tags: Vec::new(),
        }
    }

    fn keys(s: &AgentsSnapshot) -> Vec<&str> {
        s.agents.iter().map(|a| a.key.as_str()).collect()
    }

    /// The list is gathered until it's whole, then every change makes one
    /// new snapshot, whole, with a higher generation.
    #[test]
    fn a_snapshot_after_added_listed_changed_and_removed() {
        let fleet = Fleet::new();
        let none = HashMap::new();
        fleet.asked();
        fleet.apply(Frame::Added(item("t", 1, Some("notes"))), &none);
        fleet.apply(Frame::Added(item("t", 2, None)), &none);
        let before = fleet.snapshot();
        assert_eq!((before.generation, before.state, before.agents.len()), (0, ListState::Waiting, 0), "nothing until it's whole");

        fleet.apply(Frame::Listed, &none);
        let listed = fleet.snapshot();
        assert_eq!((listed.generation, &listed.state), (1, &ListState::Live));
        assert_eq!(keys(&listed), ["notes", "t#2"]);

        let mut working = item("t", 1, Some("notes"));
        working.active = true;
        fleet.apply(Frame::Changed(working), &none);
        let changed = fleet.snapshot();
        assert_eq!(changed.generation, 2);
        assert!(changed.agents[0].active);

        // A rename is the same agent, named anew.
        fleet.apply(Frame::Changed(item("t", 2, Some("links"))), &none);
        assert_eq!(keys(&fleet.snapshot()), ["notes", "links"]);

        fleet.apply(Frame::Added(item("u", 1, Some("later"))), &none);
        fleet.apply(Frame::Removed(item("t", 1, Some("notes"))), &none);
        let removed = fleet.snapshot();
        assert_eq!(keys(&removed), ["links", "later"]);
        assert_eq!(removed.generation, 5);
    }

    #[test]
    fn a_removal_of_an_agent_never_listed_is_ignored() {
        let fleet = Fleet::new();
        let none = HashMap::new();
        fleet.asked();
        fleet.apply(Frame::Added(item("t", 1, Some("notes"))), &none);
        fleet.apply(Frame::Listed, &none);
        let before = fleet.snapshot();
        fleet.apply(Frame::Removed(item("t", 9, Some("ghost"))), &none);
        let after = fleet.snapshot();
        assert_eq!(after.generation, before.generation, "nothing changed, so nothing is sent");
        assert_eq!(keys(&after), ["notes"]);
    }

    /// A refusal shows none of your agents; a failure keeps what was listed and says so.
    #[test]
    fn refused_and_failed_lists_say_so() {
        let fleet = Fleet::new();
        let none = HashMap::new();
        fleet.asked();
        fleet.apply(Frame::Added(item("t", 1, Some("notes"))), &none);
        fleet.apply(Frame::Listed, &none);
        fleet.asked();
        fleet.apply(Frame::Error(WireError(json!({ "message": "the daemon is restarting" }))), &none);
        let failed = fleet.snapshot();
        assert_eq!(failed.state, ListState::Error { message: "the daemon is restarting".into() });
        assert_eq!(keys(&failed), ["notes"]);
        fleet.asked();
        assert_eq!(fleet.snapshot().state, failed.state, "a failure stands while it's asked again");
        fleet.apply(Frame::Forbidden, &none);
        let refused = fleet.snapshot();
        assert_eq!(refused.state, ListState::Forbidden);
        assert!(refused.agents.is_empty());
    }

    /// Lists in turn: each ends after what it says, but the last, which stays open.
    struct Lists {
        lists: Mutex<Vec<Vec<Frame>>>,
        asked: Mutex<Vec<tokio::time::Instant>>,
    }

    #[async_trait]
    impl Source for Lists {
        fn list(&self, _: CancellationToken) -> Frames<Frame> {
            self.asked.lock().unwrap().push(tokio::time::Instant::now());
            let mut lists = self.lists.lock().unwrap();
            let next = lists.remove(0);
            if lists.is_empty() { Box::pin(futures::stream::iter(next).chain(futures::stream::pending())) } else { Box::pin(futures::stream::iter(next)) }
        }

        async fn template(&self, _: &str) -> Option<AgentTemplate> {
            None
        }
    }

    /// A list that ends is asked again a second later, and what it says
    /// then is the list: an agent gone meanwhile is gone.
    #[tokio::test(start_paused = true)]
    async fn a_list_that_ends_is_asked_again_a_second_later() {
        let fleet = Fleet::new();
        let source = Arc::new(Lists {
            lists: Mutex::new(vec![
                vec![Frame::Added(item("t", 1, Some("notes"))), Frame::Added(item("t", 2, Some("links"))), Frame::Listed],
                vec![Frame::Added(item("t", 2, Some("links"))), Frame::Listed],
            ]),
            asked: Mutex::new(Vec::new()),
        });
        let stop = CancellationToken::new();
        let (f, s, st) = (fleet.clone(), source.clone(), stop.clone());
        let task = tokio::spawn(async move { f.follow(s.as_ref(), st).await });
        let mut seen = fleet.subscribe();
        let again = seen.wait_for(|s| s.state == ListState::Live && s.generation >= 3).await.unwrap().clone();
        assert_eq!(keys(&again), ["links"], "notes went while it was away");
        let asked = source.asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 2);
        assert!(asked[1] - asked[0] >= AGAIN, "asked again only after a second");
        stop.cancel();
        task.await.unwrap();
    }

    /// A list that was live and ended is waiting until it's whole again.
    #[test]
    fn a_list_that_ended_is_waiting() {
        let fleet = Fleet::new();
        let none = HashMap::new();
        fleet.asked();
        fleet.apply(Frame::Added(item("t", 1, Some("notes"))), &none);
        fleet.apply(Frame::Listed, &none);
        fleet.ended();
        let ended = fleet.snapshot();
        assert_eq!(ended.state, ListState::Waiting);
        assert_eq!(keys(&ended), ["notes"], "what it last said stays on screen meanwhile");
    }
}

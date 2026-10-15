//! Your agents, live: one `agents::list` held open for the app's life.
//!
//! The daemon's list says every agent, `Listed`, then what changes, and
//! stays open. The app holds exactly one, here, and keeps what it says in
//! one map; when the list ends or fails it is asked again a second later.
//! Every change makes a whole new snapshot with a higher generation, sent
//! to whatever watches (`agents_watch`), so nothing that watches ever
//! reconciles a delta: the pattern of [`crate::tabs`].
//!
//! Each snapshot carries the same agents as who made whom ([`tree`]), with
//! how many cards each has waiting on you: the Agents map's data, built here
//! so an agent reaching the registry reads the same map a person does.

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
use crate::door::Door;
use crate::view::{AgentView, AgentsSnapshot, CardView, CreatorView, ListState, MapNode, MapWhat, OwnerView, agent_view, error_text};

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
    /// How many cards wait on you, by the agent's key.
    cards: HashMap<String, u32>,
}

/// What names a list item for good.
fn id_of(agent: &agents::list::server::response::Agent) -> String {
    format!("{}#{}", agent.template, agent.index)
}

impl Fleet {
    pub fn new() -> Arc<Fleet> {
        Arc::new(Fleet { state: Mutex::new(State::default()), tx: watch::channel(AgentsSnapshot::default()).0 })
    }

    /// Ask the daemon, and follow what it says for the app's life; and
    /// follow the door's cards, so each agent's count of them is current.
    pub fn start(self: &Arc<Self>, daemon: Arc<dyn Daemon>, door: Arc<Door>) {
        let fleet = self.clone();
        tokio::spawn(async move { fleet.follow(daemon.as_ref(), CancellationToken::new()).await });
        let fleet = self.clone();
        tokio::spawn(async move {
            let mut events = door.watch(CancellationToken::new());
            fleet.set_cards(&door.cards());
            while events.next().await.is_some() {
                fleet.set_cards(&door.cards());
            }
        });
    }

    /// The cards waiting on you now: each agent's count of them.
    pub fn set_cards(&self, cards: &[CardView]) {
        let mut counts = HashMap::new();
        for card in cards {
            *counts.entry(card.agent.clone()).or_insert(0) += 1;
        }
        let mut s = self.state.lock().unwrap();
        s.cards = counts;
        self.publish(&mut s);
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
        let tree = tree(&agents, &s.cards);
        let same = {
            let last = self.tx.borrow();
            last.state == s.list && last.agents == agents && last.tree == tree
        };
        if same {
            return;
        }
        s.generation += 1;
        self.tx.send_replace(AgentsSnapshot { generation: s.generation, state: s.list.clone(), agents, tree });
    }
}

/// Where an agent hangs in the map, by its direct maker.
enum Place {
    /// You made it: a root.
    Root,
    /// Another agent in the list made it: under that one (by position).
    Under(usize),
    /// A tool of yours made it: under that tool's node (by its key).
    Tool(String),
    /// Its maker isn't in the list, or it names itself as its maker.
    Elsewhere,
}

/// What names a tool as a maker, for its node's key.
fn tool_key(maker: &CreatorView) -> Option<String> {
    match maker {
        CreatorView::Tool { template, index, name: _ } => Some(format!("tool:{template}#{index}")),
        CreatorView::DependencyTool { template, owner } => Some(match owner {
            OwnerView::Agent { template: of, index, name: _ } => format!("tool:{template}@{of}#{index}"),
            OwnerView::AgentTemplate { template: of } => format!("tool:{template}@{of}"),
            OwnerView::Global => format!("tool:{template}@global"),
        }),
        CreatorView::Client { .. } | CreatorView::Agent { .. } => None,
    }
}

/// How a level is ordered: working first, then the most recently active,
/// then those never run; the list's order (oldest made first) breaks ties.
fn ordered(mut nodes: Vec<(MapNode, Option<chrono::DateTime<chrono::FixedOffset>>, usize)>) -> Vec<MapNode> {
    nodes.sort_by(|(a, a_at, a_i), (b, b_at, b_i)| b.working.cmp(&a.working).then(b_at.cmp(a_at)).then(a_i.cmp(b_i)));
    nodes.into_iter().map(|(node, _, _)| node).collect()
}

/// Who made whom, from the live list: the agents you made at the top, an
/// agent made by another agent under its maker, agents made by one of your
/// tools under that tool, and agents whose maker isn't in the list in one
/// group at the end. Each agent is in the map once, whatever its makers
/// say: one that names itself, or a ring of agents each made by the next,
/// is placed once and goes no deeper.
pub fn tree(agents: &[AgentView], cards: &HashMap<String, u32>) -> Vec<MapNode> {
    let at: HashMap<(&str, u64), usize> = agents.iter().enumerate().map(|(i, a)| ((a.template.as_str(), a.index), i)).collect();
    let mut under: Vec<Vec<usize>> = vec![Vec::new(); agents.len()];
    let mut roots = Vec::new();
    let mut tools: IndexMap<String, (CreatorView, Vec<usize>)> = IndexMap::new();
    let mut elsewhere = Vec::new();
    for (i, agent) in agents.iter().enumerate() {
        let place = match &agent.creator {
            CreatorView::Client { .. } => Place::Root,
            CreatorView::Agent { template, index, name: _ } => match at.get(&(template.as_str(), *index)) {
                Some(&maker) if maker != i => Place::Under(maker),
                Some(_) | None => Place::Elsewhere,
            },
            maker @ (CreatorView::Tool { .. } | CreatorView::DependencyTool { .. }) => match tool_key(maker) {
                Some(key) => Place::Tool(key),
                None => Place::Elsewhere,
            },
        };
        match place {
            Place::Root => roots.push(i),
            Place::Under(maker) => under[maker].push(i),
            Place::Tool(key) => tools.entry(key).or_insert_with(|| (agent.creator.clone(), Vec::new())).1.push(i),
            Place::Elsewhere => elsewhere.push(i),
        }
    }

    let mut placed = vec![false; agents.len()];
    // One agent and everything under it, each once. Its order key goes beside it.
    fn grow(i: usize, agents: &[AgentView], under: &[Vec<usize>], cards: &HashMap<String, u32>, placed: &mut [bool]) -> (MapNode, Option<chrono::DateTime<chrono::FixedOffset>>, usize) {
        placed[i] = true;
        let agent = &agents[i];
        let mut children = Vec::new();
        for &c in &under[i] {
            if !placed[c] {
                children.push(grow(c, agents, under, cards, placed));
            }
        }
        let at = agent.last_active.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok());
        let children = ordered(children);
        let working = agent.active || children.iter().any(|c| c.working);
        let what = MapWhat::Agent { agent: agent.clone(), cards: cards.get(&agent.key).copied().unwrap_or(0) };
        (MapNode { key: format!("agent:{}", agent.key), what, working, children }, at, i)
    }
    // A group's order key: the newest activity of anything in it.
    fn group(key: String, what: MapWhat, members: Vec<(MapNode, Option<chrono::DateTime<chrono::FixedOffset>>, usize)>) -> (MapNode, Option<chrono::DateTime<chrono::FixedOffset>>, usize) {
        let newest = members.iter().filter_map(|(_, at, _)| *at).max();
        let first = members.iter().map(|(_, _, i)| *i).min().unwrap_or(usize::MAX);
        let children = ordered(members);
        let working = children.iter().any(|c| c.working);
        (MapNode { key, what, working, children }, newest, first)
    }

    let grown = |from: Vec<usize>, placed: &mut Vec<bool>| {
        let mut out = Vec::new();
        for i in from {
            if !placed[i] {
                out.push(grow(i, agents, &under, cards, placed));
            }
        }
        out
    };
    let mut top = grown(roots, &mut placed);
    for (key, (maker, members)) in tools {
        let members = grown(members, &mut placed);
        if !members.is_empty() {
            top.push(group(key, MapWhat::Tool { maker }, members));
        }
    }
    let mut lost = grown(elsewhere, &mut placed);
    // A ring of agents each made by another in it hangs from no root: the
    // first of it in the list's order holds the rest.
    let rest = (0..agents.len()).collect();
    lost.extend(grown(rest, &mut placed));
    let mut out = ordered(top);
    if !lost.is_empty() {
        out.push(group("elsewhere".into(), MapWhat::Elsewhere, lost).0);
    }
    out
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

    // --- the tree ------------------------------------------------------------

    fn made_by(template: &str, index: u64, name: Option<&str>, maker: Creator) -> AgentView {
        let mut a = item(template, index, name);
        a.creator = maker;
        agent_view(a, None)
    }

    fn by_agent(template: &str, index: u64) -> Creator {
        Creator::Agent(creator::Agent { template: template.into(), index, name: None })
    }

    fn by_tool(template: &str, index: u64, name: Option<&str>) -> Creator {
        Creator::Tool(creator::Tool::Record { template: template.into(), index, name: name.map(Into::into) })
    }

    fn you() -> Creator {
        Creator::Client(creator::Client { identity: "you".into() })
    }

    /// The map as one line: each node's key, with what hangs under it in brackets.
    fn shape(nodes: &[MapNode]) -> String {
        nodes.iter().map(|n| if n.children.is_empty() { n.key.clone() } else { format!("{}[{}]", n.key, shape(&n.children)) }).collect::<Vec<_>>().join(" ")
    }

    fn count(nodes: &[MapNode]) -> usize {
        nodes.iter().map(|n| usize::from(matches!(n.what, MapWhat::Agent { .. })) + count(&n.children)).sum()
    }

    /// The agents you made are at the top, and an agent another made hangs
    /// under its maker, as deep as the making goes.
    #[test]
    fn makers_and_what_they_made_are_joined() {
        let agents = [
            made_by("t", 1, Some("site"), you()),
            made_by("t", 2, Some("links"), by_agent("t", 1)),
            made_by("u", 1, None, by_agent("t", 2)),
            made_by("t", 3, Some("notes"), you()),
        ];
        assert_eq!(shape(&tree(&agents, &HashMap::new())), "agent:site[agent:links[agent:u#1]] agent:notes");
    }

    /// An agent whose maker isn't in your list hangs in one group at the end.
    #[test]
    fn an_agent_whose_maker_is_not_listed_is_in_its_own_group() {
        let agents = [made_by("t", 5, Some("stray"), by_agent("gone", 9)), made_by("t", 1, Some("site"), you())];
        let map = tree(&agents, &HashMap::new());
        assert_eq!(shape(&map), "agent:site elsewhere[agent:stray]");
        assert_eq!(map[1].what, MapWhat::Elsewhere);
    }

    /// Agents one tool made hang under that tool, named as the daemon names it as their maker.
    #[test]
    fn agents_made_by_a_tool_hang_under_it() {
        let agents = [
            made_by("t", 1, Some("digest"), by_tool("tt", 1, Some("nightly-digest"))),
            made_by("t", 2, Some("digest-2"), by_tool("tt", 1, Some("nightly-digest"))),
            made_by("t", 3, None, by_tool("tt", 2, None)),
            made_by("t", 4, Some("site"), you()),
        ];
        let map = tree(&agents, &HashMap::new());
        assert_eq!(shape(&map), "tool:tt#1[agent:digest agent:digest-2] tool:tt#2[agent:t#3] agent:site");
        assert_eq!(map[0].what, MapWhat::Tool { maker: CreatorView::Tool { template: "tt".into(), index: 1, name: Some("nightly-digest".into()) } });
        let dependency = Creator::Tool(creator::Tool::Dependency { template: "dep".into(), owner: diverge_sdk::daemon::key::Owner::Global });
        assert_eq!(shape(&tree(&[made_by("t", 1, Some("x"), dependency)], &HashMap::new())), "tool:dep@global[agent:x]");
    }

    /// Working first, then the most recently active, then those never run;
    /// a tool's place is its busiest agent's.
    #[test]
    fn each_level_is_working_first_then_newest() {
        let ago = |minutes: i64| Some((Utc::now() - chrono::TimeDelta::minutes(minutes)).to_rfc3339());
        let mut never = made_by("t", 1, Some("never"), you());
        never.last_active = None;
        let mut hour = made_by("t", 2, Some("hour"), you());
        hour.last_active = ago(60);
        let mut minute = made_by("t", 3, Some("minute"), you());
        minute.last_active = ago(1);
        let mut working = made_by("t", 4, Some("working"), you());
        (working.active, working.last_active) = (true, ago(120));
        let mut digest = made_by("t", 5, Some("digest"), by_tool("tt", 1, None));
        digest.last_active = ago(30);
        let map = tree(&[never, hour, minute, working, digest], &HashMap::new());
        assert_eq!(shape(&map), "agent:working agent:minute tool:tt#1[agent:digest] agent:hour agent:never");
        assert!(map[0].working && !map[1].working);
    }

    /// An agent that names itself as its maker, or a ring of agents each
    /// made by the next, is placed once, and the walk ends.
    #[test]
    fn a_maker_that_loops_is_placed_once() {
        let agents = [
            made_by("t", 1, Some("me"), by_agent("t", 1)),
            made_by("t", 2, Some("a"), by_agent("t", 3)),
            made_by("t", 3, Some("b"), by_agent("t", 2)),
            made_by("t", 4, Some("c"), by_agent("t", 3)),
        ];
        let map = tree(&agents, &HashMap::new());
        assert_eq!(count(&map), 4, "each once: {}", shape(&map));
        assert_eq!(shape(&map), "elsewhere[agent:me agent:a[agent:b[agent:c]]]");
    }

    /// Each agent's node says how many cards wait on you, and a new count is a new snapshot.
    #[test]
    fn each_agent_carries_its_waiting_cards() {
        let fleet = Fleet::new();
        let none = HashMap::new();
        fleet.asked();
        fleet.apply(Frame::Added(item("t", 1, Some("notes"))), &none);
        fleet.apply(Frame::Listed, &none);
        let before = fleet.snapshot();
        let card = |agent: &str| CardView { id: 1, agent: agent.into(), from: None, kind: crate::view::CardKind::Question, question: "?".into(), options: vec![], at: String::new(), call: None, hire: None };
        fleet.set_cards(&[card("notes"), card("notes"), card("someone-else")]);
        let after = fleet.snapshot();
        assert_eq!(after.generation, before.generation + 1);
        assert!(matches!(&after.tree[0].what, MapWhat::Agent { cards: 2, .. }));
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

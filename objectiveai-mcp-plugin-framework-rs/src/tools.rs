//! The tools a plugin serves, swappable while it serves them.
//!
//! Tools are DECLARED with rmcp's own macros — `#[tool_router]`,
//! `#[tool]` — which produce a [`ToolRouter`]. What rmcp has no route
//! to is changing that set once a server is running: every
//! `ToolRouter` mutator takes `&mut self`, every `ServerHandler`
//! method takes `&self`, and the transport builds a fresh handler per
//! session from an `Fn` factory. Nothing in that chain can hand a
//! plugin back a way in.
//!
//! [`Tools`] is that way in. It holds the route list and one callback,
//! and it is immutable but for [`replace`][Tools::replace] — which
//! swaps the whole list and tells whoever is serving it. Hand it to
//! [`serve`][crate::serve::serve], keep a clone of the `Arc`, and call
//! `replace` whenever the set should change.
//!
//! ```no_run
//! # use objectiveai_mcp_plugin_framework::tools::Tools;
//! # use rmcp::handler::server::tool::{ToolRoute, ToolRouter};
//! # struct State;
//! # fn routes() -> Vec<ToolRoute<State>> { Vec::new() }
//! # fn fewer_routes() -> Vec<ToolRoute<State>> { Vec::new() }
//! let tools = Tools::new(routes());
//! let handle = tools.clone();
//!
//! // Later, from anywhere: the served set changes, and the client is
//! // told.
//! handle.replace(fewer_routes());
//! ```
//!
//! **No lock.** The route list is an [`ArcSwap`]: reads happen on every
//! request and are wait-free, a swap happens rarely and is a single
//! atomic store. A `RwLock` would put a lock on the hot path to make
//! the cold path marginally simpler.

use std::sync::{Arc, OnceLock};

use arc_swap::ArcSwap;
use rmcp::handler::server::tool::ToolRoute;

/// A callback run after the route list is replaced, receiving the NEW
/// list.
///
/// It is passed the routes rather than reading them back off [`Tools`]
/// for a reason that is easy to miss: a closure holding
/// `Arc<Tools<S>>` would close a cycle through the `notifier` field —
/// `Tools` owns the closure, the closure owns `Tools` — and the whole
/// structure would leak. Taking the list as an argument makes that
/// cycle unrepresentable.
type Notifier<S> = Box<dyn Fn(Arc<Vec<ToolRoute<S>>>) + Send + Sync>;

/// The set of tools a plugin is serving.
pub struct Tools<S> {
    routes: ArcSwap<Vec<ToolRoute<S>>>,
    /// Installed once by [`serve`][crate::serve::serve], never by a
    /// plugin. `OnceLock` because "set exactly once, then read from
    /// many threads" is precisely what it is for — no lock, no
    /// `Option<Mutex<_>>`.
    notifier: OnceLock<Notifier<S>>,
}

impl<S> std::fmt::Debug for Tools<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tools")
            .field("routes", &self.routes.load().len())
            .field("notifier", &self.notifier.get().is_some())
            .finish()
    }
}

impl<S> Tools<S> {
    /// The tools to start with — typically the whole of
    /// `MyServer::tool_router()`, which is
    /// [`into_iter`][IntoIterator]-able.
    pub fn new(routes: impl IntoIterator<Item = ToolRoute<S>>) -> Arc<Self> {
        Arc::new(Self {
            routes: ArcSwap::from_pointee(routes.into_iter().collect()),
            notifier: OnceLock::new(),
        })
    }

    /// Replace the served tools, wholesale.
    ///
    /// The only mutation there is. After this returns, the new set is
    /// what a request will route against, and whoever is serving has
    /// been told — which for [`serve`][crate::serve::serve] means the
    /// live router is rebuilt and the client is sent
    /// `notifications/tools/list_changed`.
    ///
    /// Wholesale rather than add/remove because a partial edit has to
    /// answer "what was there before?", and a plugin deciding its tool
    /// set from its identity and arguments already knows the answer it
    /// wants — it does not want to diff its way there.
    pub fn replace(&self, routes: impl IntoIterator<Item = ToolRoute<S>>) {
        let routes = Arc::new(routes.into_iter().collect::<Vec<_>>());
        // Store BEFORE notifying: the callback's whole job is to make
        // the world agree with this value, and it cannot do that if
        // the value is not there yet.
        self.routes.store(Arc::clone(&routes));
        if let Some(notifier) = self.notifier.get() {
            notifier(routes);
        }
    }

    /// The tools currently served.
    pub fn routes(&self) -> Arc<Vec<ToolRoute<S>>> {
        self.routes.load_full()
    }

    /// Install the callback [`replace`][Self::replace] fires.
    ///
    /// Crate-internal and once-only: `serve` claims it, and a plugin
    /// serving the same `Tools` twice would otherwise silently leave
    /// one of the two servers stale. Returns `false` if it was already
    /// claimed.
    pub(crate) fn on_replace(
        &self,
        notifier: impl Fn(Arc<Vec<ToolRoute<S>>>) + Send + Sync + 'static,
    ) -> bool {
        self.notifier.set(Box::new(notifier)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::{CallToolResult, Tool};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Server;

    fn route(name: &'static str) -> ToolRoute<Server> {
        let schema: Arc<rmcp::model::JsonObject> = Arc::new(Default::default());
        ToolRoute::new_dyn(Tool::new(name, "", schema), |_context| {
            Box::pin(async { Ok(CallToolResult::success(vec![])) })
        })
    }

    fn names(tools: &Tools<Server>) -> Vec<String> {
        tools
            .routes()
            .iter()
            .map(|route| route.attr.name.to_string())
            .collect()
    }

    #[test]
    fn replace_swaps_the_served_set() {
        let tools = Tools::new([route("alpha"), route("beta")]);
        assert_eq!(names(&tools), ["alpha", "beta"]);

        tools.replace([route("gamma")]);
        assert_eq!(names(&tools), ["gamma"], "wholesale, not merged");

        tools.replace([]);
        assert!(names(&tools).is_empty(), "and emptying is allowed");
    }

    /// The notifier gets the NEW list, once per replace. Anything else
    /// and a server wired to it would drift from what is served.
    #[test]
    fn the_notifier_receives_the_new_list_once_per_replace() {
        let tools = Tools::new([route("alpha")]);
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(ArcSwap::from_pointee(Vec::<String>::new()));

        let (calls_in, seen_in) = (calls.clone(), seen.clone());
        assert!(tools.on_replace(move |routes| {
            calls_in.fetch_add(1, Ordering::SeqCst);
            seen_in.store(Arc::new(
                routes.iter().map(|r| r.attr.name.to_string()).collect(),
            ));
        }));

        tools.replace([route("beta"), route("gamma")]);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(seen.load().as_slice(), ["beta", "gamma"]);

        tools.replace([route("delta")]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(seen.load().as_slice(), ["delta"]);
    }

    /// The store must land before the callback runs — the callback
    /// exists to bring the world into line with it.
    ///
    /// Referencing `Tools` from inside its own callback takes a
    /// [`Weak`][std::sync::Weak], never an `Arc`: an `Arc` would close
    /// the cycle the notifier signature exists to avoid. This is the
    /// supported way to look back, and the reason the new list is
    /// passed as an argument is so that you rarely need to.
    #[test]
    fn the_new_routes_are_readable_from_inside_the_notifier() {
        let tools = Tools::new([route("alpha")]);
        let observed = Arc::new(ArcSwap::from_pointee(Vec::<String>::new()));

        let weak = Arc::downgrade(&tools);
        let observed_in = observed.clone();
        tools.on_replace(move |_new| {
            let Some(live) = weak.upgrade() else { return };
            observed_in.store(Arc::new(
                live.routes()
                    .iter()
                    .map(|r| r.attr.name.to_string())
                    .collect(),
            ));
        });

        tools.replace([route("beta")]);
        assert_eq!(
            observed.load().as_slice(),
            ["beta"],
            "the swap is visible before the callback runs",
        );
    }

    #[test]
    fn replacing_without_a_notifier_is_fine() {
        let tools = Tools::new([route("alpha")]);
        tools.replace([route("beta")]);
        assert_eq!(names(&tools), ["beta"]);
    }

    /// Two servers over one `Tools` would leave one of them stale, so
    /// the second claim is refused rather than silently winning.
    #[test]
    fn the_notifier_can_only_be_claimed_once() {
        let tools: Arc<Tools<Server>> = Tools::new([]);
        assert!(tools.on_replace(|_| {}));
        assert!(!tools.on_replace(|_| {}));
    }
}

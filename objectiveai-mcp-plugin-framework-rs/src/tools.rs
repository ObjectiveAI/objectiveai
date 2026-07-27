//! A tool router a plugin can change while it is serving.
//!
//! Tools are DECLARED with rmcp's own macros — `#[tool_router]`,
//! `#[tool]` — because there is nothing wrong with them and a
//! framework that reinvented the declaration would only have to keep
//! pace with rmcp forever. What the framework adds is the other half:
//! deciding at RUNTIME which of those declared tools a client can see.
//!
//! ```no_run
//! # use objectiveai_mcp_plugin_framework::{arguments, tools::Tools};
//! # use rmcp::handler::server::tool::ToolRouter;
//! # struct MyServer { tools: Tools<MyServer> }
//! # fn build() -> ToolRouter<MyServer> { ToolRouter::new() }
//! # fn example() -> MyServer {
//! let tools = Tools::new(build());
//! // The agent decides what this plugin exposes.
//! if !arguments().contains_key("admin") {
//!     tools.remove("dangerous_thing");
//! }
//! MyServer { tools }
//! # }
//! ```
//!
//! Then delegate rmcp's two tool methods to it — `list_tools` to
//! [`Tools::list`], `call_tool` to [`Tools::call`] — instead of using
//! `#[tool_handler]`, which assumes the router is a plain field.
//!
//! **On the lock.** Every mutation takes it briefly and every call
//! CLONES the router out and releases it before doing any work. That
//! is deliberate: a tool call can run for a long time, and holding a
//! read guard across it would let one slow call block a mutation — and
//! then, because a waiting writer stops new readers, block every other
//! call behind it. Cloning costs one map of `Arc`s per call, which is
//! nothing next to the round trip a tool call already is.

use std::sync::{Arc, RwLock};

use rmcp::handler::server::tool::{ToolCallContext, ToolRoute, ToolRouter};
use rmcp::model::{CallToolResult, Tool};

/// A [`ToolRouter`] that can be edited while the server is running.
///
/// Cheap to clone — every clone shares one router, so a handler can
/// hold one and a background task another.
pub struct Tools<S> {
    inner: Arc<RwLock<ToolRouter<S>>>,
}

impl<S> Clone for Tools<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<S: Send + Sync + 'static> std::fmt::Debug for Tools<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tools").field("tools", &self.names()).finish()
    }
}

impl<S: Send + Sync + 'static> Default for Tools<S> {
    fn default() -> Self {
        Self::new(ToolRouter::new())
    }
}

impl<S: Send + Sync + 'static> Tools<S> {
    /// Wrap a router — typically the one rmcp's `#[tool_router]`
    /// generated.
    pub fn new(router: ToolRouter<S>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(router)),
        }
    }

    /// The tools a client can currently see, for `list_tools`.
    pub fn list(&self) -> Vec<Tool> {
        self.read().list_all()
    }

    /// Just the names, sorted — the cheap form of [`Self::list`].
    pub fn names(&self) -> Vec<String> {
        self.read()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    /// Whether `name` is currently routable.
    pub fn has(&self, name: &str) -> bool {
        self.read().get(name).is_some()
    }

    /// Add a route, replacing any tool of the same name.
    pub fn add(&self, route: ToolRoute<S>) {
        self.write().add_route(route);
    }

    /// Remove a tool entirely. Gone from `list_tools` and from
    /// `call_tool`, as if it had never been declared.
    ///
    /// Use [`Self::disable`] instead to keep it re-enableable.
    pub fn remove(&self, name: &str) {
        self.write().remove_route(name);
    }

    /// Hide a tool without dropping it — reversible with
    /// [`Self::enable`]. Returns whether the name was NEWLY disabled
    /// (`false` if it already was).
    ///
    /// The name does not have to exist yet. A name disabled before its
    /// route is added stays disabled when it arrives, which is what
    /// makes the useful order possible: work out what this identity
    /// and these arguments forbid, disable those names, and only then
    /// install the router rmcp generated. The forbidden tools are
    /// never briefly visible.
    ///
    /// A disabled tool is indistinguishable from an absent one to the
    /// client: missing from `list_tools`, and calling it gives the same
    /// "tool not found" an unknown name would. That is rmcp's
    /// behaviour and the right one — a client should not be able to
    /// map out what it is not allowed to reach.
    pub fn disable(&self, name: &str) -> bool {
        self.write().disable_route(name.to_string())
    }

    /// Undo [`Self::disable`]. Returns whether the name WAS disabled;
    /// `false` means it was already visible (or never disabled), not
    /// that it does not exist.
    pub fn enable(&self, name: &str) -> bool {
        self.write().enable_route(name)
    }

    /// Edit the router directly, for anything the methods above do not
    /// cover (`merge`, bulk changes, rmcp APIs added later).
    ///
    /// The lock is held for the closure, so keep it short and do not
    /// block in it.
    pub fn with_router<R>(&self, f: impl FnOnce(&mut ToolRouter<S>) -> R) -> R {
        f(&mut self.write())
    }

    /// Tell the connected client when the tool list changes, so a
    /// mutation after `initialize` is not invisible until it happens
    /// to re-list.
    pub fn notify_changes_to(&self, peer: &rmcp::service::Peer<rmcp::RoleServer>) {
        self.write().bind_peer_notifier(peer);
    }

    /// A read guard, recovering from poisoning.
    ///
    /// A panic while mutating leaves the router structurally intact —
    /// it is a map — so refusing to serve afterwards would turn one
    /// failed edit into a dead plugin.
    fn read(&self) -> std::sync::RwLockReadGuard<'_, ToolRouter<S>> {
        self.inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, ToolRouter<S>> {
        self.inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Dispatch a call, for `call_tool`.
    ///
    /// Takes a SNAPSHOT of the router and releases the lock before
    /// invoking anything — see the module docs. A tool removed while
    /// its own call is in flight therefore still finishes; the removal
    /// governs the next call, not this one.
    pub async fn call(
        &self,
        context: ToolCallContext<'_, S>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let router = self.read().clone();
        router.call(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The router is generic over the server type; these tests never
    /// call a tool, so the type only has to satisfy the bounds.
    struct Server;

    fn route(name: &'static str) -> ToolRoute<Server> {
        let schema: Arc<rmcp::model::JsonObject> = Arc::new(Default::default());
        ToolRoute::new_dyn(Tool::new(name, "", schema), |_context| {
            Box::pin(async { Ok(CallToolResult::success(vec![])) })
        })
    }

    fn tools() -> Tools<Server> {
        let tools = Tools::default();
        tools.add(route("alpha"));
        tools.add(route("beta"));
        tools
    }

    #[test]
    fn removal_is_permanent_and_disable_is_not() {
        let tools = tools();
        assert_eq!(tools.names(), ["alpha", "beta"]);

        assert!(tools.disable("beta"));
        assert_eq!(tools.names(), ["alpha"], "disabled tools are unlisted");
        assert!(!tools.has("beta"), "and unreachable while disabled");

        assert!(tools.enable("beta"));
        assert_eq!(tools.names(), ["alpha", "beta"], "and come back");

        tools.remove("beta");
        assert_eq!(tools.names(), ["alpha"]);
        assert!(
            !tools.enable("beta"),
            "enable reports the DISABLED set, and a removed tool was \
             never in it",
        );
        assert!(!tools.has("beta"), "removal is not undone by enable");
    }

    /// The order that matters for a plugin: decide what is forbidden
    /// BEFORE installing the router, so a forbidden tool is never
    /// momentarily listed.
    #[test]
    fn a_name_disabled_before_its_route_stays_disabled() {
        let tools: Tools<Server> = Tools::default();
        assert!(tools.disable("later"), "disabling an absent name is recorded");

        tools.add(route("later"));
        assert!(
            !tools.has("later"),
            "a route added under a disabled name arrives disabled",
        );
        assert!(tools.names().is_empty());

        assert!(tools.enable("later"));
        assert_eq!(tools.names(), ["later"], "and appears once enabled");
    }

    /// Clones share one router: a handler and a background task must
    /// not end up editing different tool lists.
    #[test]
    fn clones_share_one_router() {
        let tools = tools();
        let other = tools.clone();
        other.remove("alpha");
        assert_eq!(tools.names(), ["beta"]);
    }

    /// Editing an unknown tool is a no-op, not a panic — the caller is
    /// usually a config-driven loop that should not have to check.
    #[test]
    fn editing_an_unknown_tool_is_harmless() {
        let tools = tools();
        tools.remove("nonexistent");
        // Disabling records the NAME, so it reports true the first time
        // even with no such route — see `disable`.
        assert!(tools.disable("nonexistent"));
        assert!(!tools.disable("nonexistent"), "and false the second");
        assert_eq!(tools.names(), ["alpha", "beta"], "real tools untouched");
    }
}

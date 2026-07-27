//! Who this plugin is running as, read once from the environment.
//!
//! A plugin runs inside ObjectiveAI or it does not run at all. The
//! laboratory host creates one EPHEMERAL container per completion and
//! stamps the caller identity onto it at create time — the six agent
//! values from the request's transient headers, the plugin trio from
//! the canonical image coordinates, the declared arguments verbatim
//! off `X-OBJECTIVEAI-ARGUMENTS`, and the Postgres URL of the tunnel
//! the container dials. That is the whole context, and it is fixed for
//! the process's life: one container, one completion, one identity.
//!
//! Which is why this is read ONCE and cached. The environment cannot
//! change under a running plugin — nothing in ObjectiveAI rewrites it —
//! so re-reading and re-parsing per access would buy nothing and cost
//! a JSON parse each time.
//!
//! The identity itself is NOT redefined here: it is
//! [`objectiveai_sdk::identity::Identity`], the same struct the rest
//! of ObjectiveAI passes over the wire. Only the reading is ours.
//!
//! ```no_run
//! # use objectiveai_mcp_plugin_framework::identity;
//! let env = identity::environment();
//! if let Some(owner) = &env.identity.plugin_owner {
//!     println!("running as {owner}");
//! }
//! ```

use std::sync::OnceLock;

use indexmap::IndexMap;
use objectiveai_sdk::identity::Identity;

/// The plugin's declared arguments: whatever the agent configured for
/// this plugin, in DECLARATION ORDER (hence `IndexMap`, not
/// `HashMap` — the order is part of what was declared).
pub type Arguments = IndexMap<String, serde_json::Value>;

/// Everything the host told this container about itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment {
    /// The caller identity — agent coordinates, response routing, and
    /// the plugin trio naming which installed plugin this is. Every
    /// field is `Option`: the host stamps only what it has, and an
    /// ABSENT variable is meaningfully different from an empty one.
    pub identity: Identity,
    /// The arguments the agent declared for this plugin. Empty when it
    /// declared none — the same JSON that arrives as
    /// `X-OBJECTIVEAI-ARGUMENTS` on every MCP call, available here
    /// before the first call arrives.
    pub arguments: Arguments,
    /// The Postgres URL this container dials
    /// (`OBJECTIVEAI_POSTGRES_URL`), tunnelled by the laboratory host.
    /// `None` outside a plugin container.
    pub postgres_url: Option<String>,
}

/// The process's environment, parsed once.
///
/// A function rather than a `pub static` so that the caching is an
/// implementation detail: callers get a plain reference and cannot
/// depend on how (or whether) it is memoized.
pub fn environment() -> &'static Environment {
    static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();
    ENVIRONMENT.get_or_init(Environment::read)
}

impl Environment {
    /// Read the process environment directly, bypassing the cache.
    ///
    /// [`environment`] is what callers want; this is for the rare case
    /// of wanting a fresh read.
    pub fn read() -> Self {
        Self::read_from(|name| std::env::var(name).ok())
    }

    /// Read from an arbitrary lookup.
    ///
    /// Parsing is separated from the process environment so it can be
    /// tested at all: `set_var` is `unsafe` under edition 2024, and the
    /// environment is process-global, so tests that mutated it would
    /// race each other for no benefit.
    fn read_from(lookup: impl Fn(&str) -> Option<String>) -> Self {
        // Absent-or-empty collapse to `None`. Empty counts as absent
        // deliberately: the host emits only the fields it HAS
        // (`Identity::identity_env` filters out every `None`), so an
        // empty value can only have come from somewhere else — every
        // field here is "unset or a real value", never "".
        let var = |name: &str| lookup(name).filter(|v| !v.is_empty());
        Self {
            // The nine identity variables plus the task flag,
            // mirroring `Identity::identity_env` exactly — that method
            // writes this set, this reads it back.
            identity: Identity {
                agent_instance_hierarchy: var("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY"),
                agent_id: var("OBJECTIVEAI_AGENT_ID"),
                agent_full_id: var("OBJECTIVEAI_AGENT_FULL_ID"),
                agent_remote: var("OBJECTIVEAI_AGENT_REMOTE"),
                response_id: var("OBJECTIVEAI_RESPONSE_ID"),
                response_ids: var("OBJECTIVEAI_RESPONSE_IDS"),
                plugin_owner: var("OBJECTIVEAI_PLUGIN_OWNER"),
                plugin_name: var("OBJECTIVEAI_PLUGIN_NAME"),
                plugin_version: var("OBJECTIVEAI_PLUGIN_VERSION"),
                // Stamped only when true (the host CONFIG_SET_FORBIDDEN
                // convention), so absence is false rather than unknown.
                task: var("OBJECTIVEAI_TASK").as_deref() == Some("true"),
            },
            // Unparseable arguments read as EMPTY rather than
            // panicking. A plugin that cannot start is worse than one
            // that starts unconfigured — and this value is not the
            // plugin to validate: the API serialized it and the host
            // copied it through untouched, so a parse failure is a bug
            // upstream, not hostile input to defend against here.
            arguments: var("OBJECTIVEAI_ARGUMENTS")
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default(),
            postgres_url: var("OBJECTIVEAI_POSTGRES_URL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(pairs: &[(&str, &str)]) -> Environment {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Environment::read_from(|name| {
            pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
        })
    }

    #[test]
    fn empty_environment_yields_nothing_set() {
        let env = read(&[]);
        assert_eq!(env.identity, Identity::default());
        assert!(env.arguments.is_empty());
        assert_eq!(env.postgres_url, None);
    }

    /// The round trip that matters: what the host WRITES through
    /// `Identity::identity_env` is what this reads back.
    #[test]
    fn identity_env_round_trips() {
        let written = Identity {
            agent_id: Some("agent".into()),
            plugin_owner: Some("exampleorg".into()),
            plugin_name: Some("hello".into()),
            plugin_version: Some("v0.1.0".into()),
            task: true,
            ..Default::default()
        };
        let pairs = written.identity_env();
        let env = Environment::read_from(|name| {
            pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
        });
        assert_eq!(env.identity, written);
    }

    #[test]
    fn arguments_keep_declaration_order() {
        let env = read(&[(
            "OBJECTIVEAI_ARGUMENTS",
            r#"{"zebra":1,"apple":2,"mango":3}"#,
        )]);
        let keys: Vec<&str> = env.arguments.keys().map(String::as_str).collect();
        assert_eq!(keys, ["zebra", "apple", "mango"]);
    }

    /// Absence and emptiness are the same thing here, so a caller never
    /// has to special-case an empty string.
    #[test]
    fn empty_values_read_as_absent() {
        let env = read(&[("OBJECTIVEAI_AGENT_ID", ""), ("OBJECTIVEAI_TASK", "")]);
        assert_eq!(env.identity.agent_id, None);
        assert!(!env.identity.task);
    }

    #[test]
    fn unparseable_arguments_do_not_panic() {
        let env = read(&[("OBJECTIVEAI_ARGUMENTS", "not json")]);
        assert!(env.arguments.is_empty());
    }
}

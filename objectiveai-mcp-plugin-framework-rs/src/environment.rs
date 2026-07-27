//! The ambient context, read once from the environment.
//!
//! A plugin runs inside ObjectiveAI or it does not run at all. The
//! laboratory host creates one EPHEMERAL container per completion and
//! stamps that container's whole context onto it at create time — the
//! six agent values from the request's transient headers, the plugin
//! trio from the canonical image coordinates, the declared arguments
//! verbatim off `X-OBJECTIVEAI-ARGUMENTS`, and the Postgres URL of the
//! tunnel the container dials. It is fixed for the process's life: one
//! container, one completion, one identity.
//!
//! Which is why each accessor reads ONCE and caches. The environment
//! cannot change under a running plugin — nothing in ObjectiveAI
//! rewrites it — so re-reading per access would buy nothing and cost a
//! JSON parse each time. Caching per accessor rather than as one bag
//! means a plugin that never asks for its arguments never parses them.
//!
//! The identity itself is NOT redefined here: it is
//! [`objectiveai_sdk::identity::Identity`], the same struct the rest of
//! ObjectiveAI passes over the wire. Only the reading is ours.
//!
//! ```no_run
//! if let Some(owner) = &objectiveai_mcp_plugin_framework::identity().plugin_owner {
//!     println!("running as {owner}");
//! }
//! ```

use std::sync::OnceLock;

use indexmap::IndexMap;
use objectiveai_sdk::identity::Identity;

/// The plugin's declared arguments: whatever the agent configured for
/// this plugin, in DECLARATION ORDER (hence `IndexMap`, not `HashMap` —
/// the order is part of what was declared).
pub type Arguments = IndexMap<String, serde_json::Value>;

/// Who this plugin is running as: agent coordinates, response routing,
/// and the plugin trio naming which installed plugin this is.
///
/// Every field is `Option`, and an ABSENT one is meaningfully different
/// from an empty one — the host stamps only what it has. Outside a
/// laboratory container the whole thing is simply empty.
pub fn identity() -> &'static Identity {
    static IDENTITY: OnceLock<Identity> = OnceLock::new();
    IDENTITY.get_or_init(|| read_identity(env))
}

/// The arguments the agent declared for this plugin.
///
/// Empty when it declared none — the same JSON that arrives as
/// `X-OBJECTIVEAI-ARGUMENTS` on every MCP call, available here before
/// the first call arrives.
pub fn arguments() -> &'static Arguments {
    static ARGUMENTS: OnceLock<Arguments> = OnceLock::new();
    ARGUMENTS.get_or_init(|| read_arguments(env))
}

/// The Postgres URL this container dials, tunnelled by the laboratory
/// host. `None` outside a plugin container.
///
/// Crate-internal: the connection is the framework's to own, so a
/// plugin is handed a database rather than a URL to dial for itself.
#[allow(dead_code, reason = "for the framework's own database layer, still to come")]
pub(crate) fn postgres_url() -> Option<&'static str> {
    static POSTGRES_URL: OnceLock<Option<String>> = OnceLock::new();
    POSTGRES_URL
        .get_or_init(|| env("OBJECTIVEAI_POSTGRES_URL"))
        .as_deref()
}

/// One variable off the real process environment, absent-or-empty
/// collapsed to `None`.
///
/// Empty counts as absent deliberately: the host emits only the fields
/// it HAS (`Identity::identity_env` filters out every `None`), so an
/// empty value can only have come from somewhere else — every field
/// here is "unset or a real value", never `""`.
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// The nine identity variables plus the task flag, mirroring
/// [`Identity::identity_env`] exactly — that method writes this set,
/// this reads it back.
///
/// Takes the lookup rather than calling [`env`] directly so it can be
/// tested at all: `set_var` is `unsafe` under edition 2024 and the
/// environment is process-global, so tests that mutated it would race
/// each other for no benefit.
fn read_identity(lookup: impl Fn(&str) -> Option<String>) -> Identity {
    Identity {
        agent_instance_hierarchy: lookup("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY"),
        agent_id: lookup("OBJECTIVEAI_AGENT_ID"),
        agent_full_id: lookup("OBJECTIVEAI_AGENT_FULL_ID"),
        agent_remote: lookup("OBJECTIVEAI_AGENT_REMOTE"),
        response_id: lookup("OBJECTIVEAI_RESPONSE_ID"),
        response_ids: lookup("OBJECTIVEAI_RESPONSE_IDS"),
        plugin_owner: lookup("OBJECTIVEAI_PLUGIN_OWNER"),
        plugin_name: lookup("OBJECTIVEAI_PLUGIN_NAME"),
        plugin_version: lookup("OBJECTIVEAI_PLUGIN_VERSION"),
        // Stamped only when true (the host's CONFIG_SET_FORBIDDEN
        // convention), so absence is false rather than unknown.
        task: lookup("OBJECTIVEAI_TASK").as_deref() == Some("true"),
    }
}

/// `OBJECTIVEAI_ARGUMENTS`, the JSON the host copied off the header.
///
/// Unparseable arguments read as EMPTY rather than panicking. A plugin
/// that cannot start is worse than one that starts unconfigured — and
/// this value is not the plugin's to validate: the API serialized it
/// and the host copied it through untouched, so a parse failure is a
/// bug upstream, not hostile input to defend against here.
fn read_arguments(lookup: impl Fn(&str) -> Option<String>) -> Arguments {
    lookup("OBJECTIVEAI_ARGUMENTS")
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lookup over fixed pairs, applying the same absent-or-empty
    /// collapse [`env`] applies to the real environment.
    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .filter(|v| !v.is_empty())
        }
    }

    #[test]
    fn empty_environment_yields_nothing_set() {
        assert_eq!(read_identity(lookup(&[])), Identity::default());
        assert!(read_arguments(lookup(&[])).is_empty());
    }

    /// The round trip that matters: what the host WRITES through
    /// `Identity::identity_env` is what this reads back. If the host
    /// ever renames a variable, this fails rather than silently
    /// returning `None`.
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
        let read = read_identity(|name| {
            pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
        });
        assert_eq!(read, written);
    }

    #[test]
    fn arguments_keep_declaration_order() {
        let arguments = read_arguments(lookup(&[(
            "OBJECTIVEAI_ARGUMENTS",
            r#"{"zebra":1,"apple":2,"mango":3}"#,
        )]));
        let keys: Vec<&str> = arguments.keys().map(String::as_str).collect();
        assert_eq!(keys, ["zebra", "apple", "mango"]);
    }

    /// Absence and emptiness are the same thing here, so a caller never
    /// has to special-case an empty string.
    #[test]
    fn empty_values_read_as_absent() {
        let identity = read_identity(lookup(&[
            ("OBJECTIVEAI_AGENT_ID", ""),
            ("OBJECTIVEAI_TASK", ""),
        ]));
        assert_eq!(identity.agent_id, None);
        assert!(!identity.task);
    }

    #[test]
    fn unparseable_arguments_do_not_panic() {
        let arguments = read_arguments(lookup(&[("OBJECTIVEAI_ARGUMENTS", "not json")]));
        assert!(arguments.is_empty());
    }
}

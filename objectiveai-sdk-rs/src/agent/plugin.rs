//! Agent-declared plugins — ONE plugin IS ONE MCP server.
//!
//! The next-iteration plugin shape: an agent lists the plugins it
//! uses, each identified by `owner` / `name` / `version`, and each
//! plugin runs as exactly one MCP server (ultimately a container image
//! built on the laboratory host it runs on). There is no `executable`
//! flag and no per-plugin server list — the plugin IS its server.
//!
//! Coexists with the legacy [`super::ClientObjectiveaiMcp`]
//! declaration surface during the transition; the legacy surface will
//! be deleted once consumers move over.
//!
//! Content-addressed: the `plugins` field flows into each upstream's
//! `id()` hash, so [`prepare`]'s canonicalization (argument
//! normalization + sorting) is what makes two equivalent declarations
//! hash identically.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One plugin declaration: the plugin's coordinates plus optional
/// startup arguments for its MCP server.
///
/// NOTE: `name` (the repository segment) is slated for removal in a
/// later iteration of the plugin model; it remains part of the
/// identity triple for now.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.Plugin")]
pub struct Plugin {
    /// GitHub `<owner>` segment.
    pub owner: String,
    /// Repository segment.
    pub name: String,
    /// Plugin version.
    pub version: String,
    /// Optional key→value arguments handed to the plugin's MCP server
    /// at startup. `Some(value)` ⇒ `--key value`; `None` ⇒ a bare
    /// `--key` flag. The plugin author decides how to interpret them.
    /// [`prepare`] normalizes (`Some("") → None`), sorts the map by
    /// key, and collapses an empty map to `None` so two equivalent
    /// declarations canonicalize to byte-identical JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_indexmap_string_option_string)]
    pub arguments: Option<IndexMap<String, Option<String>>>,
}

impl PartialOrd for Plugin {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Plugin {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Coordinates first. `IndexMap` doesn't derive `Ord`, so the
        // arguments compare by walking entries in iteration order —
        // deterministic after `prepare`'s `sort_keys` pass. `None`
        // sorts before `Some(...)` via the standard `Option<T>::cmp`.
        let by_coord = (&self.owner, &self.name, &self.version).cmp(&(
            &other.owner,
            &other.name,
            &other.version,
        ));
        if by_coord.is_ne() {
            return by_coord;
        }
        let a: Option<Vec<(&String, &Option<String>)>> =
            self.arguments.as_ref().map(|m| m.iter().collect());
        let b: Option<Vec<(&String, &Option<String>)>> =
            other.arguments.as_ref().map(|m| m.iter().collect());
        a.cmp(&b)
    }
}

impl Plugin {
    /// `owner`, `name`, and `version` must all be non-empty, and every
    /// `arguments` key (if present) must be non-empty (values may be
    /// empty — they canonicalize to bare flags in [`prepare`]).
    pub fn validate(&self) -> Result<(), String> {
        if self.owner.is_empty() {
            return Err("`owner` cannot be empty".into());
        }
        if self.name.is_empty() {
            return Err("`name` cannot be empty".into());
        }
        if self.version.is_empty() {
            return Err("`version` cannot be empty".into());
        }
        if let Some(args) = self.arguments.as_ref() {
            for (k, _) in args {
                if k.is_empty() {
                    return Err("`arguments` key cannot be empty".into());
                }
            }
        }
        Ok(())
    }
}

/// Validates an agent's `plugins` list: each entry validates, and no
/// two entries share an `(owner, name, version)` triple — one
/// instance of a plugin per agent. Free-function counterpart to
/// [`super::client_objectiveai_mcp::validate`].
pub fn validate(plugins: &[Plugin]) -> Result<(), String> {
    for plugin in plugins {
        plugin.validate()?;
    }
    for (i, a) in plugins.iter().enumerate() {
        for b in &plugins[i + 1..] {
            if a.owner == b.owner && a.name == b.name && a.version == b.version
            {
                return Err(format!(
                    "`plugins` contains duplicate entry: \"{}/{}@{}\"",
                    a.owner, a.name, a.version,
                ));
            }
        }
    }
    Ok(())
}

/// Canonicalization pass for an agent's `plugins` list: lowercase
/// `owner` and `name` (GitHub owner/repo lookups are case-insensitive,
/// and the laboratory host derives container/image identity from the
/// lowercased pair; the content-addressing impact of mixed-case
/// declarations collapsing is accepted). `version` keeps its case —
/// it maps to a git tag, and git refs are case-SENSITIVE (semver
/// permits uppercase prerelease identifiers like `v1.2.3-RC1`). Then
/// normalize each entry's `arguments` (`Some("") → None` values,
/// key-sort, collapse an empty map to `None`), and sort the list by
/// `(owner, name, version, arguments)`. The enclosing field uses
/// `skip_serializing_if = "Vec::is_empty"`, so an empty list needs no
/// collapse here.
pub fn prepare(mut plugins: Vec<Plugin>) -> Vec<Plugin> {
    for plugin in &mut plugins {
        plugin.owner = plugin.owner.to_lowercase();
        plugin.name = plugin.name.to_lowercase();
        let drop_empty = match plugin.arguments.as_mut() {
            Some(args) => {
                for (_, v) in args.iter_mut() {
                    if let Some(s) = v.as_deref() {
                        if s.is_empty() {
                            *v = None;
                        }
                    }
                }
                args.sort_keys();
                args.is_empty()
            }
            None => false,
        };
        if drop_empty {
            plugin.arguments = None;
        }
    }
    plugins.sort();
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(
        owner: &str,
        name: &str,
        version: &str,
        args: &[(&str, Option<&str>)],
    ) -> Plugin {
        let arguments = if args.is_empty() {
            None
        } else {
            let mut m = IndexMap::new();
            for (k, v) in args {
                m.insert(k.to_string(), v.map(|s| s.to_string()));
            }
            Some(m)
        };
        Plugin {
            owner: owner.into(),
            name: name.into(),
            version: version.into(),
            arguments,
        }
    }

    #[test]
    fn prepare_canonicalizes_arguments_and_sorts() {
        let a = prepare(vec![
            plugin("b", "x", "1", &[]),
            plugin("a", "x", "1", &[("z", Some("1")), ("a", Some(""))]),
        ]);
        assert_eq!(a[0].owner, "a");
        let args = a[0].arguments.as_ref().unwrap();
        // key-sorted, empty-string value normalized to bare flag
        assert_eq!(
            args.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["a", "z"],
        );
        assert_eq!(args.get("a").unwrap(), &None);
    }

    #[test]
    fn prepare_collapses_empty_arguments_to_none() {
        let a = prepare(vec![Plugin {
            owner: "o".into(),
            name: "n".into(),
            version: "1".into(),
            arguments: Some(IndexMap::new()),
        }]);
        assert!(a[0].arguments.is_none());
    }

    #[test]
    fn equivalent_declarations_canonicalize_identically() {
        let a = prepare(vec![plugin(
            "o",
            "n",
            "1",
            &[("b", Some("1")), ("a", Some("2"))],
        )]);
        let b = prepare(vec![plugin(
            "o",
            "n",
            "1",
            &[("a", Some("2")), ("b", Some("1"))],
        )]);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
        );
    }

    #[test]
    fn prepare_lowercases_owner_and_name_but_not_version() {
        let a = prepare(vec![plugin("Acme", "Widgets", "v1.2.3-RC1", &[])]);
        let b = prepare(vec![plugin("acme", "widgets", "v1.2.3-RC1", &[])]);
        assert_eq!(a[0].owner, "acme");
        assert_eq!(a[0].name, "widgets");
        // version case is PRESERVED — git tags are case-sensitive
        assert_eq!(a[0].version, "v1.2.3-RC1");
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
        );
    }

    #[test]
    fn validate_rejects_duplicate_coordinates() {
        let err = validate(&[
            plugin("o", "n", "1", &[]),
            plugin("o", "n", "1", &[("k", Some("v"))]),
        ])
        .expect_err("duplicate coordinates must be rejected");
        assert!(err.contains("duplicate"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_empty_fields() {
        assert!(validate(&[plugin("", "n", "1", &[])]).is_err());
        assert!(validate(&[plugin("o", "", "1", &[])]).is_err());
        assert!(validate(&[plugin("o", "n", "", &[])]).is_err());
        assert!(validate(&[plugin("o", "n", "1", &[("", Some("v"))])]).is_err());
    }
}

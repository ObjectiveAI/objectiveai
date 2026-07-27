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
    Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema, arbitrary::Arbitrary,
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
    /// at startup. Values are free-form JSON: a string behaves as
    /// `--key value` and `null` as a bare `--key` flag, but an object,
    /// an array or a number is equally valid and the plugin author
    /// decides how to interpret them.
    ///
    /// [`prepare`] normalizes (an empty STRING becomes `null`, so the
    /// two spellings of a valueless flag canonicalize together), sorts
    /// the map by key AND every object key nested inside a value at
    /// any depth, and collapses an empty map to `None` — so two
    /// equivalent declarations serialize byte-identically, which is
    /// what makes an agent id content-addressable. Array element order
    /// is left alone: that is data, not spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_indexmap_string_json_value)]
    pub arguments: Option<IndexMap<String, serde_json::Value>>,
}

impl Plugin {
    /// The total order [`prepare`] sorts by: coordinates first, then
    /// the arguments as their JSON text.
    ///
    /// A KEY rather than an `Ord` impl because `serde_json::Value` is
    /// neither `Ord` nor `Eq` — it holds `f64`, which has no total
    /// order — so `Plugin` cannot have one either. Comparing the
    /// serialized text instead is well-defined and, after
    /// `prepare`'s `sort_keys` pass, deterministic.
    ///
    /// Coordinates are joined with NUL, which cannot appear in any of
    /// them, so no boundary is ambiguous — `("a", "b/c")` and
    /// `("a/b", "c")` cannot collide into one key.
    fn sort_key(&self) -> String {
        let arguments = self
            .arguments
            .as_ref()
            .and_then(|arguments| serde_json::to_string(arguments).ok())
            .unwrap_or_default();
        format!(
            "{}\0{}\0{}\0{arguments}",
            self.owner, self.name, self.version,
        )
    }

    /// `owner`, `name`, and `version` must all be non-empty; `version`
    /// must start with `v` — it IS the plugin repo's git tag, Go-modules
    /// style (`v1.2.3`), byte-for-byte with no rewriting anywhere
    /// downstream. Every `arguments` key (if present) must be non-empty
    /// (values may be empty — they canonicalize to bare flags in
    /// [`prepare`]).
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
        if !self.version.starts_with('v') {
            return Err(format!(
                "`version` {:?} must start with 'v' — it is the plugin repo's git tag, Go-modules style (v1.2.3)",
                self.version,
            ));
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
/// Sort every object key in `value`, at every depth — inside nested
/// objects and inside arrays alike.
///
/// Needed because `serde_json` runs with `preserve_order` here, so a
/// `Map` keeps INSERTION order: `{"b":1,"a":2}` and `{"a":2,"b":1}`
/// are equal as data but serialize to different bytes, and the agent
/// id is a hash of those bytes. Sorting only the TOP level would leave
/// `{"opts":{"b":1,"a":2}}` uncanonical.
///
/// Array ELEMENT order is deliberately untouched: an array is ordered
/// data, and reordering it would change what the plugin was told, not
/// just how it was spelled. (This is the difference from the
/// `deep_sort` in the api tests, which reorders arrays because it
/// exists to compare two documents ignoring order.)
fn sort_object_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (_, nested) in map.iter_mut() {
                sort_object_keys(nested);
            }
            map.sort_keys();
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_object_keys(item);
            }
        }
        _ => {}
    }
}

pub fn prepare(mut plugins: Vec<Plugin>) -> Vec<Plugin> {
    for plugin in &mut plugins {
        plugin.owner = plugin.owner.to_lowercase();
        plugin.name = plugin.name.to_lowercase();
        let drop_empty = match plugin.arguments.as_mut() {
            Some(args) => {
                for (_, v) in args.iter_mut() {
                    // An empty string and `null` are two spellings of
                    // the same valueless flag; canonicalize on `null`.
                    if v.as_str() == Some("") {
                        *v = serde_json::Value::Null;
                    }
                    sort_object_keys(v);
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
    // By CACHED key: the key serializes the arguments to JSON, so
    // computing it per comparison would re-serialize every plugin
    // O(log n) times. `sort_by_cached_key` computes it once each.
    plugins.sort_by_cached_key(Plugin::sort_key);
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `None` ⇒ JSON null (the bare flag); `Some(s)` ⇒ a JSON string.
    fn json_value(v: Option<&str>) -> serde_json::Value {
        match v {
            Some(s) => serde_json::Value::String(s.to_string()),
            None => serde_json::Value::Null,
        }
    }

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
                // `None` is the bare flag, i.e. a JSON null.
                m.insert(k.to_string(), json_value(*v));
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
        assert_eq!(args.get("a").unwrap(), &serde_json::Value::Null);
    }

    /// The point of widening: a value is any JSON, not just a string.
    #[test]
    fn arguments_carry_arbitrary_json() {
        let mut arguments = IndexMap::new();
        arguments.insert("retries".to_string(), serde_json::json!(3));
        arguments.insert("verbose".to_string(), serde_json::json!(true));
        arguments.insert("hosts".to_string(), serde_json::json!(["a", "b"]));
        arguments.insert("nested".to_string(), serde_json::json!({"k": "v"}));
        let prepared = prepare(vec![Plugin {
            owner: "o".into(),
            name: "n".into(),
            version: "v1".into(),
            arguments: Some(arguments),
        }]);
        let args = prepared[0].arguments.as_ref().unwrap();
        assert_eq!(args.get("retries").unwrap(), &serde_json::json!(3));
        assert_eq!(args.get("hosts").unwrap(), &serde_json::json!(["a", "b"]));
        assert_eq!(args.get("nested").unwrap(), &serde_json::json!({"k": "v"}));
        // Still key-sorted.
        assert_eq!(
            args.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["hosts", "nested", "retries", "verbose"],
        );
    }

    /// Object keys canonicalize at EVERY depth, including inside
    /// arrays — a top-level-only sort would leave two equal documents
    /// hashing differently, and the agent id is that hash.
    #[test]
    fn prepare_sorts_object_keys_at_every_depth() {
        let mut arguments = IndexMap::new();
        arguments.insert(
            "opts".to_string(),
            serde_json::json!({"z": 1, "a": {"y": 2, "b": 3}}),
        );
        arguments.insert(
            "list".to_string(),
            serde_json::json!([{"q": 1, "p": 2}]),
        );
        let prepared = prepare(vec![Plugin {
            owner: "o".into(),
            name: "n".into(),
            version: "v1".into(),
            arguments: Some(arguments),
        }]);
        let args = prepared[0].arguments.as_ref().unwrap();
        assert_eq!(
            serde_json::to_string(args).unwrap(),
            r#"{"list":[{"p":2,"q":1}],"opts":{"a":{"b":3,"y":2},"z":1}}"#,
        );
    }

    /// Two spellings of the same document canonicalize to identical
    /// bytes however deeply the difference is buried — which is the
    /// whole point, since the id is a hash of those bytes.
    #[test]
    fn deeply_nested_orderings_canonicalize_identically() {
        let build = |value: serde_json::Value| {
            let mut arguments = IndexMap::new();
            arguments.insert("k".to_string(), value);
            prepare(vec![Plugin {
                owner: "o".into(),
                name: "n".into(),
                version: "v1".into(),
                arguments: Some(arguments),
            }])
        };
        let a = build(serde_json::json!({"outer": [{"b": 1, "a": {"d": 2, "c": 3}}]}));
        let b = build(serde_json::json!({"outer": [{"a": {"c": 3, "d": 2}, "b": 1}]}));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
        );
    }

    /// Array ELEMENT order is data, not spelling — canonicalization
    /// must not touch it.
    #[test]
    fn prepare_preserves_array_element_order() {
        let mut arguments = IndexMap::new();
        arguments.insert("hosts".to_string(), serde_json::json!(["c", "a", "b"]));
        let prepared = prepare(vec![Plugin {
            owner: "o".into(),
            name: "n".into(),
            version: "v1".into(),
            arguments: Some(arguments),
        }]);
        assert_eq!(
            prepared[0].arguments.as_ref().unwrap().get("hosts").unwrap(),
            &serde_json::json!(["c", "a", "b"]),
        );
    }

    /// Sorting is by the serialized text, since JSON values have no
    /// total order — but it must still be a deterministic one.
    #[test]
    fn prepare_orders_same_coordinates_by_arguments() {
        let a = prepare(vec![
            plugin("o", "n", "v1", &[("k", Some("b"))]),
            plugin("o", "n", "v1", &[("k", Some("a"))]),
        ]);
        let values: Vec<_> = a
            .iter()
            .map(|p| p.arguments.as_ref().unwrap().get("k").unwrap().clone())
            .collect();
        assert_eq!(values, vec![serde_json::json!("a"), serde_json::json!("b")]);
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
            plugin("o", "n", "v1", &[]),
            plugin("o", "n", "v1", &[("k", Some("v"))]),
        ])
        .expect_err("duplicate coordinates must be rejected");
        assert!(err.contains("duplicate"), "unexpected error: {err}");
    }

    #[test]
    fn validate_rejects_empty_fields() {
        assert!(validate(&[plugin("", "n", "v1", &[])]).is_err());
        assert!(validate(&[plugin("o", "", "v1", &[])]).is_err());
        assert!(validate(&[plugin("o", "n", "", &[])]).is_err());
        assert!(validate(&[plugin("o", "n", "v1", &[("", Some("v"))])]).is_err());
    }

    /// The version IS the git tag, Go-modules style: it must arrive
    /// `v`-prefixed — nothing downstream rewrites it.
    #[test]
    fn validate_requires_v_prefixed_version() {
        assert!(validate(&[plugin("o", "n", "1.2.3", &[])]).is_err());
        // Uppercase `V` is NOT the Go convention — tags are
        // case-sensitive and the required prefix is lowercase `v`.
        assert!(validate(&[plugin("o", "n", "V1.2.3", &[])]).is_err());
        assert!(validate(&[plugin("o", "n", "v1.2.3", &[])]).is_ok());
        assert!(validate(&[plugin("o", "n", "v1.2.3-RC1", &[])]).is_ok());
    }
}

//! The laboratory image spec — inline Containerfile or split
//! registry reference.
//!
//! A laboratory's base image is either INLINE (Containerfile text the
//! host builds with `podman build`, empty context) or a REGISTRY
//! reference carried as its parts — `registry` + `name` + (`tag` XOR
//! `digest`) — everywhere: the CLI
//! command surface, the daemon↔host channel, container labels, list
//! echoes. The fully-joined reference string
//! (`registry/name:tag` / `registry/name@digest`) deliberately does
//! not exist outside the laboratory host's podman internals, so an
//! unqualified short name (podman would silently resolve it against
//! docker.io) is unrepresentable end to end.
//!
//! Field rules follow the docker/OCI reference grammar (the same
//! rules the `docker-image` crate encodes):
//! - `registry`: lowercase domain components (`[a-z0-9]` with interior
//!   `.`/`_`/`-`), and it must be UNAMBIGUOUSLY a registry — contain a
//!   `.`, carry a `:port`, or be exactly `localhost`.
//! - `name`: `/`-separated path components, each
//!   `[a-z0-9]+([._-][a-z0-9]+)*`.
//! - `tag`: `[A-Za-z0-9_][A-Za-z0-9._-]{0,127}`.
//! - `digest`: `<algorithm>:<64 hex>`, e.g. `sha256:…`.
//!
//! `tag` and `digest` are mutually exclusive BY TYPE
//! ([`LaboratoryImagePin`]): the reference grammar technically allows
//! `name:tag@digest`, but the tag is decorative there (resolution is
//! by digest), so this API refuses to model the ambiguity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One laboratory base image: inline Containerfile text, or a split
/// registry reference. Untagged — the variants' keys are disjoint
/// (`containerfile` vs `registry`/`name`), mirroring the agent
/// inline-vs-remote idiom.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "laboratories.LaboratoryImage")]
pub enum LaboratoryImage {
    /// The image spec provided inline as Containerfile content. The
    /// host builds it on EVERY create (podman's layer cache still
    /// applies) against an empty context — `COPY`/`ADD` of local
    /// files fails by construction.
    #[schemars(title = "Inline")]
    Inline(InlineLaboratoryImage),
    /// A reference to an image hosted on a registry.
    #[schemars(title = "Registry")]
    Registry(RegistryLaboratoryImage),
}

impl LaboratoryImage {
    /// Validate the spec. `Err` carries a human-readable reason.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            LaboratoryImage::Inline(inline) => inline.validate(),
            LaboratoryImage::Registry(registry) => registry.validate(),
        }
    }
}

/// The Containerfile-embedded label rides `podman create`'s argv
/// (Windows caps a command line at ~32K chars), so inline content is
/// bounded to keep the worst-case-escaped label inside it.
pub const MAX_CONTAINERFILE_BYTES: usize = 16 * 1024;

/// An inline image spec: Containerfile/Dockerfile CONTENT, plain text.
///
/// The Containerfile's own `FROM` line is deliberately NOT validated
/// for full qualification — the file is the user's content, and
/// podman's build-time resolution rules apply to it verbatim.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "laboratories.InlineLaboratoryImage")]
pub struct InlineLaboratoryImage {
    /// Containerfile content (plain text — never nested/escaped JSON).
    pub containerfile: String,
}

impl InlineLaboratoryImage {
    /// Non-empty and within [`MAX_CONTAINERFILE_BYTES`].
    pub fn validate(&self) -> Result<(), String> {
        if self.containerfile.trim().is_empty() {
            return Err("containerfile must not be empty".to_string());
        }
        if self.containerfile.len() > MAX_CONTAINERFILE_BYTES {
            return Err(format!(
                "containerfile is {} bytes; max {MAX_CONTAINERFILE_BYTES} \
                 (it rides the container label on podman's argv)",
                self.containerfile.len()
            ));
        }
        Ok(())
    }
}

/// One registry-hosted laboratory base image, split into its
/// reference parts.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "laboratories.RegistryLaboratoryImage")]
pub struct RegistryLaboratoryImage {
    /// The image host — e.g. `docker.io`, `ghcr.io`,
    /// `registry.example.com:5000`, `localhost:5000`. Required and
    /// validated to be unambiguously a registry, so short-name
    /// resolution never happens.
    pub registry: String,
    /// The repository path — e.g. `library/bash`, `myorg/myimage`.
    pub name: String,
    /// Exactly one of `tag` / `digest`.
    #[serde(flatten)]
    pub pin: LaboratoryImagePin,
}

/// Manual, because the derived path cannot enforce the pin's
/// exclusivity: `pin` is a `#[serde(flatten)]`ed externally-tagged
/// enum, and serde's buffered flatten deserialization takes the FIRST
/// matching key from the remaining map — a payload carrying BOTH
/// `tag` and `digest` would silently win as `tag`. This impl rejects
/// that (and a payload carrying neither) instead. Serialization stays
/// derived — the wire shape is unchanged.
impl<'de> Deserialize<'de> for RegistryLaboratoryImage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            registry: String,
            name: String,
            #[serde(default)]
            tag: Option<String>,
            #[serde(default)]
            digest: Option<String>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let pin = match (wire.tag, wire.digest) {
            (Some(tag), None) => LaboratoryImagePin::Tag(tag),
            (None, Some(digest)) => LaboratoryImagePin::Digest(digest),
            (Some(_), Some(_)) => {
                return Err(serde::de::Error::custom(
                    "`tag` and `digest` are mutually exclusive",
                ));
            }
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "one of `tag` / `digest` is required",
                ));
            }
        };
        Ok(Self {
            registry: wire.registry,
            name: wire.name,
            pin,
        })
    }
}

/// The image's version pin: a floating `tag` or a content-addressed
/// `digest` — mutually exclusive by construction.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "laboratories.LaboratoryImagePin")]
#[serde(rename_all = "snake_case")]
pub enum LaboratoryImagePin {
    /// `registry/name:tag`.
    #[schemars(title = "Tag")]
    Tag(String),
    /// `registry/name@digest` — content-addressed, immutable.
    #[schemars(title = "Digest")]
    Digest(String),
}

impl RegistryLaboratoryImage {
    /// Validate every part against the reference grammar. `Err`
    /// carries a human-readable reason naming the offending part.
    pub fn validate(&self) -> Result<(), String> {
        validate_registry(&self.registry)?;
        validate_name(&self.name)?;
        match &self.pin {
            LaboratoryImagePin::Tag(tag) => validate_tag(tag),
            LaboratoryImagePin::Digest(digest) => validate_digest(digest),
        }
    }
}

/// One lowercase component: `[a-z0-9]+([._-][a-z0-9]+)*`.
fn is_lower_component(s: &str) -> bool {
    let mut prev_sep = true; // leading separator is invalid
    for c in s.chars() {
        match c {
            'a'..='z' | '0'..='9' => prev_sep = false,
            '.' | '_' | '-' => {
                if prev_sep {
                    return false;
                }
                prev_sep = true;
            }
            _ => return false,
        }
    }
    !s.is_empty() && !prev_sep // trailing separator is invalid
}

fn validate_registry(registry: &str) -> Result<(), String> {
    let (host, port) = match registry.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (registry, None),
    };
    if let Some(port) = port {
        if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!(
                "registry '{registry}': port must be digits after ':'"
            ));
        }
    }
    if !is_lower_component(host) {
        return Err(format!(
            "registry '{registry}': host must be lowercase alphanumerics \
             with interior '.', '_' or '-'"
        ));
    }
    // The load-bearing rule: the registry must be UNAMBIGUOUSLY a
    // registry, never a name podman would short-name-resolve. A
    // domain dot, an explicit port, or literal `localhost` qualifies.
    if !host.contains('.') && port.is_none() && host != "localhost" {
        return Err(format!(
            "registry '{registry}' is not a fully-qualified image host — \
             expected a domain (e.g. docker.io), a host:port, or localhost"
        ));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_string());
    }
    for component in name.split('/') {
        if !is_lower_component(component) {
            return Err(format!(
                "name '{name}': component '{component}' must be lowercase \
                 alphanumerics with interior '.', '_' or '-'"
            ));
        }
    }
    Ok(())
}

fn validate_tag(tag: &str) -> Result<(), String> {
    let mut chars = tag.chars();
    let valid_first = matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphanumeric() || c == '_'
    );
    if !valid_first
        || tag.len() > 128
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!(
            "tag '{tag}': expected [A-Za-z0-9_][A-Za-z0-9._-]{{0,127}}"
        ));
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), String> {
    let err = || {
        format!(
            "digest '{digest}': expected '<algorithm>:<64 hex>' \
             (e.g. sha256:…)"
        )
    };
    let Some((algorithm, hex)) = digest.split_once(':') else {
        return Err(err());
    };
    if !is_lower_component(algorithm)
        || hex.len() != 64
        || !hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(err());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(registry: &str, name: &str, pin: LaboratoryImagePin) -> LaboratoryImage {
        LaboratoryImage::Registry(RegistryLaboratoryImage {
            registry: registry.to_string(),
            name: name.to_string(),
            pin,
        })
    }

    fn tag(t: &str) -> LaboratoryImagePin {
        LaboratoryImagePin::Tag(t.to_string())
    }

    #[test]
    fn accepts_fully_qualified_forms() {
        for (registry, name) in [
            ("docker.io", "library/bash"),
            ("ghcr.io", "org/img"),
            ("registry.example.com:5000", "team/app"),
            ("localhost", "img"),
            ("localhost:5000", "a/b/c"),
        ] {
            image(registry, name, tag("latest")).validate().unwrap();
        }
        image(
            "docker.io",
            "library/bash",
            LaboratoryImagePin::Digest(format!("sha256:{}", "a".repeat(64))),
        )
        .validate()
        .unwrap();
    }

    #[test]
    fn rejects_short_name_registries() {
        // A bare word would be short-name-resolved by podman — the
        // exact silent-docker.io fallback this type exists to forbid.
        for registry in ["myorg", "bash", "registry", ""] {
            image(registry, "img", tag("latest")).validate().unwrap_err();
        }
    }

    #[test]
    fn rejects_bad_parts() {
        image("docker.io", "Library/bash", tag("latest")).validate().unwrap_err();
        image("docker.io", "library//bash", tag("latest")).validate().unwrap_err();
        image("docker.io", "bash", tag(".dot-first")).validate().unwrap_err();
        image("docker.io", "bash", tag(&"t".repeat(129))).validate().unwrap_err();
        image(
            "docker.io",
            "bash",
            LaboratoryImagePin::Digest("sha256:short".to_string()),
        )
        .validate()
        .unwrap_err();
        image("docker.io:", "bash", tag("latest")).validate().unwrap_err();
    }

    #[test]
    fn inline_validates_and_roundtrips() {
        let inline = LaboratoryImage::Inline(InlineLaboratoryImage {
            containerfile: "FROM docker.io/library/bash:latest\n".to_string(),
        });
        inline.validate().unwrap();
        // Untagged: inline serializes to just {containerfile}, and the
        // enum picks the right variant back.
        let json = serde_json::to_value(&inline).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "containerfile": "FROM docker.io/library/bash:latest\n" })
        );
        assert_eq!(
            serde_json::from_value::<LaboratoryImage>(json).unwrap(),
            inline
        );
        // Empty and oversized are rejected.
        LaboratoryImage::Inline(InlineLaboratoryImage {
            containerfile: "  ".to_string(),
        })
        .validate()
        .unwrap_err();
        LaboratoryImage::Inline(InlineLaboratoryImage {
            containerfile: "x".repeat(MAX_CONTAINERFILE_BYTES + 1),
        })
        .validate()
        .unwrap_err();
    }

    #[test]
    fn wire_shape_is_flat_and_exclusive() {
        let json = serde_json::to_value(image("docker.io", "library/bash", tag("latest")))
            .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "registry": "docker.io",
                "name": "library/bash",
                "tag": "latest",
            })
        );
        // Both pins present must not deserialize.
        assert!(
            serde_json::from_value::<LaboratoryImage>(serde_json::json!({
                "registry": "docker.io",
                "name": "bash",
                "tag": "latest",
                "digest": format!("sha256:{}", "a".repeat(64)),
            }))
            .is_err()
        );
    }
}

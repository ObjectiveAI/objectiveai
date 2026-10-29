//! The six official agent images, and what each one's settings may be.
//!
//! Each image ships `/DIVERGE.md`, whose frontmatter is its name and the JSON
//! Schema of its `arguments` (`schema_for!` over its loop's `Agent`), and the provider
//! answers a container scope's `info` with it (`shared::containers::info`).
//! No image is built or reached yet, so each loop's file is compiled in here
//! from the SDK's source and read with the SDK's own parser. When he changes
//! an image's settings and writes its file again, this app's form changes
//! with it at the next build. Nothing is run and nothing is installed to get
//! them.

use std::sync::OnceLock;

use serde_json::Value;

use diverge_sdk::daemon::endpoints::agents::create::client::request::{Image, Reference};
use diverge_sdk::shared::containers::info::Document;

/// Which of the six. The image name is `diverge-agentic-loop-<key>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Cc,
    Codex,
    Eliza,
    Hermes,
    Openrouter,
    Python,
}

pub const ALL: [Kind; 6] = [Kind::Cc, Kind::Codex, Kind::Hermes, Kind::Openrouter, Kind::Eliza, Kind::Python];

/// No image has been built or published yet, so there is no digest to name.
pub const UNBUILT_DIGEST: &str = "unbuilt";

/// Nor a registry to fetch one from: a reference still carries the image's
/// name, which is how an agent's template says which of the six it is.
pub const UNPUBLISHED_REGISTRY: &str = "unpublished";

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Cc => "cc",
            Kind::Codex => "codex",
            Kind::Eliza => "eliza",
            Kind::Hermes => "hermes",
            Kind::Openrouter => "openrouter",
            Kind::Python => "python",
        }
    }

    pub fn image_name(self) -> String {
        format!("diverge-agentic-loop-{}", self.key())
    }

    pub fn from_image_name(name: &str) -> Option<Kind> {
        ALL.into_iter().find(|kind| kind.image_name() == name)
    }

    /// Unbuilt, but one image apiece: a template's id leaves the image's
    /// references out, so two of the six may not share a digest.
    pub fn digest(self) -> String {
        format!("{UNBUILT_DIGEST}-{}", self.key())
    }

    /// The image as an agent's template names it: its digest, and its name
    /// in a reference.
    pub fn image(self) -> Image {
        Image { digest: self.digest(), references: vec![Reference { registry: UNPUBLISHED_REGISTRY.into(), name: self.image_name() }] }
    }

    /// Which of the six an image is, by the name a reference gives it.
    pub fn of_image(image: &Image) -> Option<Kind> {
        image.references.iter().find_map(|r| Kind::from_image_name(&r.name))
    }

    /// The image's `/DIVERGE.md`, as its loop ships it.
    fn document(self) -> &'static str {
        match self {
            Kind::Cc => include_str!("../../../diverge-agentic-loop-cc/DIVERGE.md"),
            Kind::Codex => include_str!("../../../diverge-agentic-loop-codex/DIVERGE.md"),
            Kind::Eliza => include_str!("../../../diverge-agentic-loop-eliza/DIVERGE.md"),
            Kind::Hermes => include_str!("../../../diverge-agentic-loop-hermes/DIVERGE.md"),
            Kind::Openrouter => include_str!("../../../diverge-agentic-loop-openrouter/DIVERGE.md"),
            Kind::Python => include_str!("../../../diverge-agentic-loop-python/DIVERGE.md"),
        }
    }

    /// The JSON Schema of the image's `arguments`.
    pub fn schema(self) -> Value {
        Document::parse(self.document()).map(|document| mended(document.schema)).unwrap_or(Value::Null)
    }

    /// Whether `arguments` is an agent this image accepts: its schema holds.
    /// The image itself checks the same at the start of a run, before it says
    /// anything; the words here are the schema's, not the image's.
    pub fn check(self, arguments: &Value) -> Result<(), String> {
        static CHECKERS: [OnceLock<Result<jsonschema::Validator, String>>; 6] = [const { OnceLock::new() }; 6];
        let checker = CHECKERS[self as usize].get_or_init(|| jsonschema::validator_for(&self.schema()).map_err(|error| error.to_string()));
        match checker {
            Ok(checker) => match checker.iter_errors(arguments).next() {
                Some(error) => Err(error.to_string()),
                None => Ok(()),
            },
            Err(error) => Err(error.clone()),
        }
    }
}

/// The files as committed carry three characters written as UTF-8 and read
/// back as Windows-1252 (— … →); put back, so the form's hints read as the
/// loops' own doc comments do. Nothing to do once the files are written again.
fn mended(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(
            text.replace("\u{e2}\u{20ac}\u{201d}", "\u{2014}").replace("\u{e2}\u{20ac}\u{a6}", "\u{2026}").replace("\u{e2}\u{2020}\u{2019}", "\u{2192}"),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(mended).collect()),
        Value::Object(members) => Value::Object(members.into_iter().map(|(key, value)| (key, mended(value))).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check is the schema's, and it refuses: what is no agent at all,
    /// and an agent missing what its image requires.
    #[test]
    fn settings_that_are_no_agent_are_refused() {
        for kind in ALL {
            assert!(kind.check(&serde_json::json!("an agent")).is_err(), "{}", kind.key());
        }
        assert!(Kind::Hermes.check(&serde_json::json!({ "model": "nousresearch/hermes-4-70b" })).is_err(), "Hermes needs a provider");
    }

    /// Writes every schema next to the bindings so a person can read them.
    #[test]
    fn every_schema_compiles_and_is_an_object() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings/schemas");
        std::fs::create_dir_all(&dir).unwrap();
        for kind in ALL {
            let schema = kind.schema();
            assert!(schema.is_object(), "{} schema is not an object", kind.key());
            std::fs::write(dir.join(format!("{}.json", kind.key())), serde_json::to_string_pretty(&schema).unwrap()).unwrap();
        }
    }
}

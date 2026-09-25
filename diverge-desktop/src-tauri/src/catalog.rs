//! The six official agent images, and what each one's settings may be.
//!
//! Each agentic-loop crate's `agent` module is self-contained (serde,
//! schemars, indexmap and the provider SDK's `Tool`), so it is compiled in
//! here straight from Ronald's source: the schema is `schema_for!(Agent)`,
//! exactly what the image's own `GET /schema` serves. When he changes an
//! image's settings, this app's form changes with it at the next build.
//! Nothing is run and nothing is installed to get them.
//!
//! A real client cannot reach an image's `GET /schema` — it answers only
//! inside its container — which is why this is compiled in. See
//! `QUESTIONS_FOR_RONALD.md`.

#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-cc/src/agent/mod.rs"]
mod cc;
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-codex/src/agent/mod.rs"]
mod codex;
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-eliza/src/agent/mod.rs"]
mod eliza;
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-hermes/src/agent/mod.rs"]
mod hermes;
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-openrouter/src/agent/mod.rs"]
mod openrouter;
#[allow(dead_code, unused_imports, clippy::all)]
#[path = "../../../diverge-agentic-loop-python/src/agent/mod.rs"]
mod python;

use serde_json::Value;

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

    /// The JSON Schema of the image's `arguments`.
    pub fn schema(self) -> Value {
        let schema = match self {
            Kind::Cc => schemars::schema_for!(cc::Agent),
            Kind::Codex => schemars::schema_for!(codex::Agent),
            Kind::Eliza => schemars::schema_for!(eliza::Agent),
            Kind::Hermes => schemars::schema_for!(hermes::Agent),
            Kind::Openrouter => schemars::schema_for!(openrouter::Agent),
            Kind::Python => schemars::schema_for!(python::Agent),
        };
        serde_json::to_value(schema).unwrap_or(Value::Null)
    }

    /// Whether `arguments` is an agent this image accepts — what the image
    /// itself checks at the start of a run, before it says anything.
    pub fn check(self, arguments: &Value) -> Result<(), String> {
        let value = arguments.clone();
        let result = match self {
            Kind::Cc => serde_json::from_value::<cc::Agent>(value).map(drop),
            Kind::Codex => serde_json::from_value::<codex::Agent>(value).map(drop),
            Kind::Eliza => serde_json::from_value::<eliza::Agent>(value).map(drop),
            Kind::Hermes => serde_json::from_value::<hermes::Agent>(value).map(drop),
            Kind::Openrouter => serde_json::from_value::<openrouter::Agent>(value).map(drop),
            Kind::Python => serde_json::from_value::<python::Agent>(value).map(drop),
        };
        result.map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

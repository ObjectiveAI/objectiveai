//! Which document, and its type.

use serde_json::Value;

use super::Error;

/// One of the three documents in the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// The daemon's.
    Daemon,
    /// The provider's.
    Provider,
    /// The CLI's own.
    Cli,
}

impl Which {
    /// The file's name in the root.
    pub fn file(self) -> &'static str {
        match self {
            Which::Daemon => "daemon.yaml",
            Which::Provider => "provider.yaml",
            Which::Cli => "cli.yaml",
        }
    }

    /// The document as its program holds it when there is no file:
    /// the type's built-in default, as JSON.
    pub fn default(self) -> Result<Value, Error> {
        let value = match self {
            Which::Daemon => serde_json::to_value(diverge_daemon::config::Config::default()),
            Which::Provider => serde_json::to_value(diverge_provider::config::Config::default()),
            Which::Cli => serde_json::to_value(crate::config::Config::default()),
        };
        value.map_err(Error::Json)
    }

    /// The value as the document's own type takes it, and as that
    /// type writes it: the YAML to put in the file. A value the type
    /// refuses — a key it does not have, a wrong kind — is
    /// [`Error::Invalid`], with the path to what was wrong.
    pub fn validate(self, value: Value) -> Result<String, Error> {
        let yaml = match self {
            Which::Daemon => {
                serde_yaml_ng::to_string(&typed::<diverge_daemon::config::Config>(value)?)
            }
            Which::Provider => {
                serde_yaml_ng::to_string(&typed::<diverge_provider::config::Config>(value)?)
            }
            Which::Cli => serde_yaml_ng::to_string(&typed::<crate::config::Config>(value)?),
        };
        yaml.map_err(Error::Serialize)
    }
}

/// The value as `T`, or where in it `T` refused.
fn typed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, Error> {
    serde_path_to_error::deserialize(value).map_err(Error::Invalid)
}

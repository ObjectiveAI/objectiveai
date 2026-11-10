//! The secrets the agent declares: the names its plugins read, each
//! to its mode, and the rotating ones with where they are left.

use diverge_sdk::shared::containers::secrets::{Declaration, Mode};

use super::Agent;
use super::plugin::{Rotates, Secret};

/// One rotating secret: its name, and where the rotated value lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotating {
    /// The setting name, and the secret's name as the template
    /// declares it.
    pub name: String,
    /// Where the plugin leaves the rotated value.
    pub rotates: Rotates,
}

impl Agent {
    /// The declaration the registration answers with: every plugin's
    /// `secrets`, on either list, the model providers' first, each
    /// name once in the position it was first named — `fixed` for a
    /// bare name or `rotates: false`, `rotating` otherwise. A name
    /// named both ways is `rotating`: the plugin that rotates it
    /// needs the value held, and the one that only reads it loses
    /// nothing by that. Nothing the harness implies of its own is
    /// here; its passphrase and salt are the lineage's.
    pub fn declaration(&self) -> Declaration {
        let mut declaration = Declaration::new();
        for plugin in self.model_provider_plugins.iter().chain(&self.plugins) {
            for secret in &plugin.secrets {
                let (name, mode) = match secret {
                    Secret::Static(name) => (name, Mode::Fixed),
                    Secret::Rotating {
                        key,
                        rotates: Rotates::InPlace(false),
                    } => (key, Mode::Fixed),
                    Secret::Rotating { key, .. } => (key, Mode::Rotating),
                };
                match declaration.get_mut(name) {
                    Some(held) => {
                        if mode == Mode::Rotating {
                            *held = Mode::Rotating;
                        }
                    }
                    None => {
                        declaration.insert(name.clone(), mode);
                    }
                }
            }
        }
        declaration
    }

    /// The rotating secrets, each with where its plugin leaves the
    /// rotated value, deduplicated by name in the same order as the
    /// declaration: the first entry naming a secret decides where it
    /// is read back from.
    pub fn rotating(&self) -> Vec<Rotating> {
        let mut rotating: Vec<Rotating> = Vec::new();
        for plugin in self.model_provider_plugins.iter().chain(&self.plugins) {
            for secret in &plugin.secrets {
                match secret {
                    Secret::Static(_)
                    | Secret::Rotating {
                        rotates: Rotates::InPlace(false),
                        ..
                    } => {}
                    Secret::Rotating { key, rotates } => {
                        if !rotating.iter().any(|held| &held.name == key) {
                            rotating.push(Rotating {
                                name: key.clone(),
                                rotates: rotates.clone(),
                            });
                        }
                    }
                }
            }
        }
        rotating
    }
}

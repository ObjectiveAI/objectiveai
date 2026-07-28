//! What a plugin gets to decide about serving.
//!
//! Two kinds of thing live here. The port and the keep-alive shape the
//! SERVER; everything else is the plugin's `initialize` reply, which
//! ObjectiveAI keeps verbatim and reports through
//! `agents mcp servers list`.
//!
//! Most of the wire shape is fixed by the laboratory host — see
//! [`crate::serve`] — and a knob for something that can only hold one
//! value is just a way to get it wrong. These are the ones that
//! genuinely vary.

use std::time::Duration;

use rmcp::model::Icon;

/// How to serve, and how to introduce yourself.
///
/// [`name`][Self::name] and [`version`][Self::version] are constructor
/// arguments rather than optional fields because they are not
/// decoration: the proxy DERIVES its routing prefix from them, so they
/// decide the tool names the agent actually sees. The rest are
/// reporting only.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// The port the plugin's manifest declares under `mcp.port`.
    ///
    /// The host publishes exactly that port, so this and the manifest
    /// are two halves that have to agree: a mismatch means the host
    /// publishes a port nothing is listening on, and the plugin looks
    /// dead rather than misconfigured.
    pub port: u16,

    /// The server name — load-bearing, not cosmetic.
    ///
    /// The proxy builds each upstream's ROUTING PREFIX from it
    /// (`_` and `.` normalize to `-`), and prepends `<prefix>_` to
    /// every tool name the agent sees. It is also the key the
    /// agent-SDK runners file this server under in their `mcpServers`
    /// config.
    ///
    /// Prefixes escalate only as far as uniqueness demands: `name`,
    /// then `name-version` on collision, then `name-version-index`.
    /// Two plugins sharing a name AND version therefore fall through
    /// to a positional index — legal, but unreadable and tied to sort
    /// order. Pick a distinct name.
    pub name: String,

    /// The server version — the prefix collision tie-breaker (see
    /// [`name`][Self::name]), and reported by `servers/list`.
    pub version: String,

    /// Human-readable title. Reporting only.
    pub title: Option<String>,

    /// What this server is. Reporting only.
    pub description: Option<String>,

    /// Where to read more. Reporting only.
    pub website_url: Option<String>,

    /// Icons for a UI to show. Reporting only.
    pub icons: Option<Vec<Icon>>,

    /// How a model should use this server — the one field here written
    /// FOR the agent rather than about the plugin. Surfaced through
    /// `agents mcp servers list`.
    pub instructions: Option<String>,

    /// How often to ping an idle SSE stream, or `None` to never.
    ///
    /// Worth having because the connection crosses podman's port
    /// publish and whatever sits between: an idle stream can be
    /// reaped by something in the middle that has no idea the plugin
    /// is simply thinking.
    pub sse_keep_alive: Option<Duration>,
}

impl Config {
    /// The config for a plugin whose manifest declares
    /// `mcp.port = port`, identifying itself as `name`/`version`.
    ///
    /// Both identifiers are required for the reason on
    /// [`name`][Self::name]: defaulting them would silently hand every
    /// plugin the same routing prefix.
    pub fn new(port: u16, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            port,
            name: name.into(),
            version: version.into(),
            title: None,
            description: None,
            website_url: None,
            icons: None,
            instructions: None,
            sse_keep_alive: Some(Duration::from_secs(15)),
        }
    }

    /// Set the human-readable title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set what this server is.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set where to read more.
    pub fn with_website_url(mut self, website_url: impl Into<String>) -> Self {
        self.website_url = Some(website_url.into());
        self
    }

    /// Set the icons a UI may show.
    pub fn with_icons(mut self, icons: impl IntoIterator<Item = Icon>) -> Self {
        self.icons = Some(icons.into_iter().collect());
        self
    }

    /// Set how a model should use this server.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two that decide the agent-visible tool names are the two a
    /// caller cannot forget.
    #[test]
    fn identity_is_required_and_the_rest_is_not() {
        let config = Config::new(8080, "hello-channel", "0.1.6");
        assert_eq!(config.port, 8080);
        assert_eq!(config.name, "hello-channel");
        assert_eq!(config.version, "0.1.6");
        assert_eq!(config.title, None);
        assert_eq!(config.description, None);
        assert_eq!(config.website_url, None);
        assert_eq!(config.icons, None);
        assert_eq!(config.instructions, None);
    }

    #[test]
    fn the_optional_reporting_fields_are_settable() {
        let config = Config::new(1, "n", "v")
            .with_title("Title")
            .with_description("What it does")
            .with_website_url("https://objectiveai.dev")
            .with_instructions("Call greet.");
        assert_eq!(config.title.as_deref(), Some("Title"));
        assert_eq!(config.description.as_deref(), Some("What it does"));
        assert_eq!(config.website_url.as_deref(), Some("https://objectiveai.dev"));
        assert_eq!(config.instructions.as_deref(), Some("Call greet."));
    }
}

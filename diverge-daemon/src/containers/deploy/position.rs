//! A declared dependency as a template, and its position: the agent
//! and that template.

use diverge_sdk::daemon::endpoints::agents::create::client::request::Image;
use diverge_sdk::daemon::endpoints::tools::routes::Path;
use diverge_sdk::daemon::endpoints::tools::templates::{Template, ToolType};
use diverge_sdk::shared::containers::tools::Tool;

use crate::store::hash;

/// The id of the tool template a declared dependency is: the
/// template with the dependency's image, limits and arguments, no
/// description and no mounts — a dependency declares none of either —
/// hashed as every template is. A dependency naming no template of
/// that id on record is unmet.
pub fn template_of(tool: &Tool) -> Result<String, serde_json::Error> {
    let template: Template = Template {
        r#type: ToolType::Tool,
        description: None,
        image: Image {
            name: tool.image.name.clone(),
            digest: tool.image.digest.clone(),
        },
        memory: tool.memory,
        disk: tool.disk,
        arguments: tool.arguments.clone(),
    };
    hash::template_id(&template)
}

/// no position a route can name.
pub fn position(root: Option<&str>, template: &str) -> Option<Path> {
    let agent = root?;
    Some(Path {
        agent: agent.to_string(),
        template: template.to_string(),
    })
}

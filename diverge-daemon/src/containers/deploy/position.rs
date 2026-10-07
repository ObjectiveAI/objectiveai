//! A declared dependency as a template, and its position in the
//! chain.

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
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        arguments: tool.arguments.clone(),
    };
    hash::template_id(&template)
}

/// The position of a dependency: the agent whose run began the chain,
/// by name, and the templates down the chain so far with the
/// dependency's own last. A chain that began at a nameless agent has
/// no position a route can name.
pub fn position(root: Option<&str>, chain: &[String], template: &str) -> Option<Path> {
    let agent = root?;
    let mut templates = chain.to_vec();
    templates.push(template.to_string());
    Some(Path {
        agent: agent.to_string(),
        templates,
    })
}

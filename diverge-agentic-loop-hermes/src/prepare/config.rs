//! `config.yaml`, rendered from the plan.

use serde_json::{Map, Value, json};

use super::{MCP_PROXY_NAME, MCP_PROXY_URL, Plan};

/// The whole configuration Hermes reads at startup, as one JSON
/// document (JSON is YAML, and Hermes parses YAML).
///
/// - `model`: the provider's id and the model; `api_key` for the
///   `custom` provider alone. Selection is THIS key — never
///   `auth.json`'s `active_provider`.
/// - `security.protected_instruction_files: false`: the second of
///   the yolo trio (the first is the environment's
///   `HERMES_YOLO_MODE`, the third is on the MCP entry below).
/// - `web`, `tts`, `image_gen`, `video_gen`: the backend pins the
///   toolsets asked for, and only those.
/// - `platform_toolsets.api_server`: the explicit membership list.
/// - `mcp_servers`: the container's own proxy, trusted in full, with
///   elicitation AND sampling off — neither has anyone to answer it.
///
/// Nothing else: session retention pruning stays at Hermes's
/// default (off), memory limits stay at Hermes's, and the config
/// version is left for Hermes to stamp.
pub fn render(plan: &Plan) -> Value {
    let mut model = Map::new();
    model.insert("provider".to_string(), Value::String(plan.provider.clone()));
    model.insert("default".to_string(), Value::String(plan.model.clone()));
    if let Some(api_key) = &plan.api_key {
        model.insert("api_key".to_string(), Value::String(api_key.clone()));
    }

    let mut config = Map::new();
    config.insert("model".to_string(), Value::Object(model));
    config.insert(
        "security".to_string(),
        json!({ "protected_instruction_files": false }),
    );

    let mut web = Map::new();
    if let Some(backend) = plan.search_backend {
        web.insert("search_backend".to_string(), Value::String(backend.into()));
    }
    if let Some(backend) = plan.extract_backend {
        web.insert("extract_backend".to_string(), Value::String(backend.into()));
    }
    if !web.is_empty() {
        config.insert("web".to_string(), Value::Object(web));
    }
    if plan.tts_elevenlabs {
        config.insert("tts".to_string(), json!({ "provider": "elevenlabs" }));
    }
    if plan.image_gen {
        config.insert("image_gen".to_string(), json!({ "provider": "fal" }));
    }
    if plan.video_gen {
        config.insert("video_gen".to_string(), json!({ "provider": "fal" }));
    }

    config.insert(
        "platform_toolsets".to_string(),
        json!({ "api_server": plan.toolsets }),
    );
    config.insert(
        "mcp_servers".to_string(),
        json!({
            MCP_PROXY_NAME: {
                "url": MCP_PROXY_URL,
                "trust": "full",
                "elicitation": { "enabled": false },
                "sampling": { "enabled": false },
            }
        }),
    );

    Value::Object(config)
}

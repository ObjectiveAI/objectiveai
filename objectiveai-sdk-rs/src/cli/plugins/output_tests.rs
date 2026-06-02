use super::*;
use serde_json::json;

fn roundtrip(out: &PluginOutput) -> serde_json::Value {
    let s = serde_json::to_string(out).unwrap();
    let back: PluginOutput = serde_json::from_str(&s).unwrap();
    serde_json::to_value(&back).unwrap()
}

#[test]
fn error_wire_shape() {
    let out = PluginOutput::Typed(TypedPluginOutput::Error(Error {
        r#type: ErrorType::Error,
        level: Some(Level::Error),
        fatal: Some(true),
        message: "plugin blew up".into(),
    }));
    let v = roundtrip(&out);
    assert_eq!(v["type"], "error");
    assert_eq!(v["level"], "error");
    assert_eq!(v["fatal"], true);
    assert_eq!(v["message"], "plugin blew up");
}

#[test]
fn notification_wire_shape() {
    let out = PluginOutput::Notification(json!({"foo": "bar", "n": 42}));
    let v = roundtrip(&out);
    // Untagged catch-all: the value's keys are the wire shape, no
    // `type:"notification"` envelope.
    assert_eq!(v["foo"], "bar");
    assert_eq!(v["n"], 42);
}

#[test]
fn command_wire_shape() {
    let out = PluginOutput::Typed(TypedPluginOutput::Command {
        id: None,
        command: "ping".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "command");
    assert_eq!(v["command"], "ping");
}

#[test]
fn mcp_wire_shape() {
    let out = PluginOutput::Typed(TypedPluginOutput::Mcp(Mcp {
        url: "https://example.com/mcp".into(),
    }));
    let v = roundtrip(&out);
    assert_eq!(v["type"], "mcp");
    assert_eq!(v["url"], "https://example.com/mcp");
}

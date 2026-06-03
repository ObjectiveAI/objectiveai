use super::*;
use serde_json::json;

fn roundtrip(out: &Output) -> serde_json::Value {
    let s = serde_json::to_string(out).unwrap();
    let back: Output = serde_json::from_str(&s).unwrap();
    serde_json::to_value(&back).unwrap()
}

#[test]
fn error_wire_shape() {
    let out = Output::Typed(TypedOutput::Error(Error {
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
    let out = Output::Notification(json!({"foo": "bar", "n": 42}));
    let v = roundtrip(&out);
    // Untagged catch-all: the value's keys are the wire shape, no
    // `type:"notification"` envelope.
    assert_eq!(v["foo"], "bar");
    assert_eq!(v["n"], 42);
}

#[test]
fn command_wire_shape() {
    let out = Output::Typed(TypedOutput::Command {
        id: "cmd-1".to_string(),
        command: "ping".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "command");
    assert_eq!(v["command"], "ping");
    assert_eq!(v["id"], "cmd-1");
}

#[test]
fn mcp_wire_shape() {
    let out = Output::Typed(TypedOutput::Mcp(Mcp {
        url: "https://example.com/mcp".into(),
        headers: None,
    }));
    let v = roundtrip(&out);
    assert_eq!(v["type"], "mcp");
    assert_eq!(v["url"], "https://example.com/mcp");
    assert!(v.get("headers").is_none());
}

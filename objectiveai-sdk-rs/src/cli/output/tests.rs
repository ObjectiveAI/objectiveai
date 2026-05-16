use std::sync::Arc;

use super::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Mutex;

fn roundtrip<T>(out: &Output<T>) -> serde_json::Value
where
    T: Serialize + serde::de::DeserializeOwned,
{
    let s = serde_json::to_string(out).unwrap();
    let back: Output<T> = serde_json::from_str(&s).unwrap();
    serde_json::to_value(&back).unwrap()
}

fn notif<T>(value: T) -> Output<T> {
    Output::Notification(Notification { value })
}

#[tokio::test]
async fn emit_via_stdout_handle() {
    // Smoke test that `Handle::Stdout` routes emit() without panicking.
    // We can't intercept stdout from a unit test, so just confirm the
    // call completes and the future is Send + 'static-safe.
    let out: Output<Ok> = notif(OK);
    out.emit(&Handle::Stdout).await;
}

#[tokio::test]
async fn emit_via_collect_handle_appends_to_vec() {
    let vec = Arc::new(Mutex::new(Vec::new()));
    let handle = Handle::Collect(vec.clone());

    notif(OK).emit(&handle).await;
    Output::<Ok>::Error(Error {
        level: Level::Warn,
        fatal: false,
        message: "heads up".into(),
    })
    .emit(&handle)
    .await;

    let snapshot = vec.lock().await;
    assert_eq!(snapshot.len(), 2);

    // First: a notification carrying the `Ok` ack — nested under `value`.
    let first = serde_json::to_value(&snapshot[0]).unwrap();
    assert_eq!(first["type"], "notification");
    assert_eq!(first["value"]["ok"], true);

    // Second: a warn-level non-fatal error.
    let second = serde_json::to_value(&snapshot[1]).unwrap();
    assert_eq!(second["type"], "error");
    assert_eq!(second["level"], "warn");
    assert_eq!(second["fatal"], false);
    assert_eq!(second["message"], "heads up");
}

#[test]
fn error_fatal_wire_shape() {
    let out: Output<Ok> = Output::Error(Error {
        level: Level::Error,
        fatal: true,
        message: "favorite not found: foo".into(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "error");
    assert_eq!(v["level"], "error");
    assert_eq!(v["fatal"], true);
    assert_eq!(v["message"], "favorite not found: foo");
}

#[test]
fn error_non_fatal_warn_wire_shape() {
    let out: Output<Ok> = Output::Error(Error {
        level: Level::Warn,
        fatal: false,
        message: "auto-update error: ...".into(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "error");
    assert_eq!(v["level"], "warn");
    assert_eq!(v["fatal"], false);
}

#[test]
fn ack_ok_wire_shape() {
    let out = notif(OK);
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["ok"], true);
}

#[test]
fn begin_wire_shape() {
    let out: Output<Ok> = Output::Begin;
    let v = roundtrip(&out);
    assert_eq!(v["type"], "begin");
    // No other fields.
    assert_eq!(v.as_object().unwrap().len(), 1);
}

#[test]
fn end_wire_shape() {
    let out: Output<Ok> = Output::End;
    let v = roundtrip(&out);
    assert_eq!(v["type"], "end");
    assert_eq!(v.as_object().unwrap().len(), 1);
}

#[test]
fn cleared_wire_shape() {
    let out = notif(Cleared { cleared: 7 });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["cleared"], 7);
}

#[test]
fn instructions_wire_shape() {
    let out = notif(Instructions {
        instructions: "follow these steps".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["instructions"], "follow these steps");
}

#[test]
fn items_generic_wire_shape() {
    #[derive(Serialize, Deserialize, Debug)]
    struct Sample {
        n: u32,
    }
    let out = notif(Items {
        items: vec![Sample { n: 1 }, Sample { n: 2 }],
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["items"][0]["n"], 1);
    assert_eq!(v["value"]["items"][1]["n"], 2);
}

#[test]
fn pair_list_item_untagged_dispatch() {
    // A Favorite-shaped pair list item should deserialize without an
    // explicit tag, by virtue of `#[serde(untagged)]`.
    let item: PairListItem = serde_json::from_value(json!({
        "name": "fav",
        "function": {"remote": "github", "owner": "o", "repository": "r", "commit": "c"},
        "profile":  {"remote": "github", "owner": "o", "repository": "r", "commit": "c"},
        "note": ""
    }))
    .unwrap();
    let out = notif(Items { items: vec![item] });
    let v = roundtrip(&out);
    assert_eq!(v["value"]["items"][0]["name"], "fav");
}

#[test]
fn jq_results_wire_shape() {
    let out = notif(JqResults {
        jq: json!([{"a": 1}, {"b": 2}]),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["jq"][0]["a"], 1);
}

#[test]
fn log_content_json_wire_shape() {
    let out = notif(LogContent::Json {
        content: json!({"completion": {"id": "abc"}}),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["content"]["completion"]["id"], "abc");
}

#[test]
fn log_content_data_url_wire_shape() {
    let out = notif(LogContent::DataUrl {
        content_data_url: "data:image/png;base64,abc".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["content_data_url"], "data:image/png;base64,abc");
}

#[test]
fn log_stream_ready_wire_shape() {
    let out = notif(LogStreamReady {
        log_stream_ready: "abc-123".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["log_stream_ready"], "abc-123");
}

#[test]
fn published_wire_shape() {
    let out = notif(Published {
        sha: "deadbeef".to_string(),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["sha"], "deadbeef");
}

#[test]
fn schema_wire_shape() {
    let out = notif(Schema {
        schema: json!({"$schema": "...", "type": "object"}),
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["schema"]["$schema"], "...");
    // `type` inside the schema lives at value.schema.type — fully
    // separate from the outer `type`:"notification" discriminator,
    // which is exactly the collision this nesting solves.
    assert_eq!(v["value"]["schema"]["type"], "object");
}

#[test]
fn schemas_list_wire_shape() {
    let out = notif(Schemas {
        schemas: vec!["Foo".to_string(), "Bar".to_string()],
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["schemas"][0], "Foo");
    assert_eq!(v["value"]["schemas"][1], "Bar");
}

#[test]
fn value_generic_wire_shape() {
    let out = notif(Value {
        value: vec!["a".to_string(), "b".to_string()],
    });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["value"][0], "a");
}

#[test]
fn detached_wire_shape() {
    let out = notif(Detached { pid: 12345 });
    let v = roundtrip(&out);
    assert_eq!(v["type"], "notification");
    assert_eq!(v["value"]["pid"], 12345);
}


use super::AssistantMessageExpression;

#[test]
fn deserialize_empty_object() {
    let json = "{}";
    let result: Result<AssistantMessageExpression, _> =
        serde_json::from_str(json);
    assert!(
        result.is_ok(),
        "failed to deserialize empty object: {}",
        result.unwrap_err()
    );
}

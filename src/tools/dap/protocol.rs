use crate::tools::lsp::json::JsonValue;
use std::collections::BTreeMap;

pub fn make_response(
    req_seq: i64,
    command: &str,
    body: Option<JsonValue>,
    seq: &mut i64,
) -> JsonValue {
    *seq += 1;
    let mut map = BTreeMap::new();
    map.insert("seq".to_string(), JsonValue::Number(*seq as f64));
    map.insert(
        "type".to_string(),
        JsonValue::String("response".to_string()),
    );
    map.insert("request_seq".to_string(), JsonValue::Number(req_seq as f64));
    map.insert(
        "command".to_string(),
        JsonValue::String(command.to_string()),
    );
    map.insert("success".to_string(), JsonValue::Bool(true));
    if let Some(b) = body {
        map.insert("body".to_string(), b);
    }
    JsonValue::Object(map)
}

pub fn make_error_response(req_seq: i64, command: &str, message: &str, seq: &mut i64) -> JsonValue {
    *seq += 1;
    let mut map = BTreeMap::new();
    map.insert("seq".to_string(), JsonValue::Number(*seq as f64));
    map.insert(
        "type".to_string(),
        JsonValue::String("response".to_string()),
    );
    map.insert("request_seq".to_string(), JsonValue::Number(req_seq as f64));
    map.insert(
        "command".to_string(),
        JsonValue::String(command.to_string()),
    );
    map.insert("success".to_string(), JsonValue::Bool(false));
    map.insert(
        "message".to_string(),
        JsonValue::String(message.to_string()),
    );
    JsonValue::Object(map)
}

pub fn make_event(event_name: &str, body: Option<JsonValue>, seq: &mut i64) -> JsonValue {
    *seq += 1;
    let mut map = BTreeMap::new();
    map.insert("seq".to_string(), JsonValue::Number(*seq as f64));
    map.insert("type".to_string(), JsonValue::String("event".to_string()));
    map.insert(
        "event".to_string(),
        JsonValue::String(event_name.to_string()),
    );
    if let Some(b) = body {
        map.insert("body".to_string(), b);
    }
    JsonValue::Object(map)
}

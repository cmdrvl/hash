use serde_json::{Map, Value};

pub const HASH_VERSION: &str = "hash.v0";

pub fn set_hash_version(record: &mut Map<String, Value>) {
    record.insert("version".to_owned(), Value::String(HASH_VERSION.to_owned()));
}

pub fn mark_skipped(record: &mut Map<String, Value>) {
    record.insert("_skipped".to_owned(), Value::Bool(true));
    record.insert("bytes_hash".to_owned(), Value::Null);
    record.insert("hash_algorithm".to_owned(), Value::Null);
}

pub fn merge_tool_versions(record: &mut Map<String, Value>) {
    let mut tool_versions = record
        .remove("tool_versions")
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();

    tool_versions.insert(
        "hash".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );
    record.insert("tool_versions".to_owned(), Value::Object(tool_versions));
}

pub fn apply_upstream_skipped_passthrough(record: &mut Map<String, Value>) {
    set_hash_version(record);
    mark_skipped(record);
    merge_tool_versions(record);
}

/// Check if a record is already marked as skipped by upstream tools
pub fn is_skipped(record: &Map<String, Value>) -> bool {
    record
        .get("_skipped")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Process a record that's already skipped - pass through without hashing
pub fn process_skipped_record(mut record: Value) -> Value {
    let Some(map) = record.as_object_mut() else {
        return record;
    };

    // Update version to hash.v0
    set_hash_version(map);

    // Ensure hash fields are null (required by schema)
    map.insert("bytes_hash".to_owned(), Value::Null);
    map.insert("hash_algorithm".to_owned(), Value::Null);

    // Update tool_versions but preserve existing _skipped and _warnings
    update_tool_versions(map);

    record
}

/// Process a normal record by adding hash fields
pub fn process_hashed_record(mut record: Value, bytes_hash: String, algorithm: &str) -> Value {
    let Some(map) = record.as_object_mut() else {
        return record;
    };

    // Update version to hash.v0
    set_hash_version(map);

    // Set hash fields
    map.insert("bytes_hash".to_owned(), Value::String(bytes_hash));
    map.insert(
        "hash_algorithm".to_owned(),
        Value::String(algorithm.to_string()),
    );

    // Update tool_versions
    update_tool_versions(map);

    record
}

/// Update tool_versions map with hash version
pub fn update_tool_versions(record: &mut Map<String, Value>) {
    let mut tool_versions = record
        .get("tool_versions")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    tool_versions.insert(
        "hash".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );

    record.insert("tool_versions".to_owned(), Value::Object(tool_versions));
}

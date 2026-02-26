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

pub fn process_file_io_error(record: Value, error: impl Into<String>) -> Value {
    let path = record
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let error = error.into();
    process_io_failed_record(record, &path, &error)
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

/// Process a record where file hashing failed due to IO error
pub fn process_io_failed_record(mut record: Value, path: &str, io_error: &str) -> Value {
    let Some(map) = record.as_object_mut() else {
        return record;
    };

    // Update version to hash.v0
    set_hash_version(map);

    // Mark as skipped with null hash fields
    map.insert("_skipped".to_owned(), Value::Bool(true));
    map.insert("bytes_hash".to_owned(), Value::Null);
    map.insert("hash_algorithm".to_owned(), Value::Null);

    // Append IO failure warning
    append_io_warning(map, path, io_error);

    // Update tool_versions
    update_tool_versions(map);

    record
}

/// Append an IO warning to the _warnings array
fn append_io_warning(record: &mut Map<String, Value>, path: &str, error: &str) {
    let mut warnings = record
        .get("_warnings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let warning = serde_json::json!({
        "tool": "hash",
        "code": "E_IO",
        "message": "Cannot read file",
        "detail": {
            "path": path,
            "error": error
        }
    });

    warnings.push(warning);
    record.insert("_warnings".to_owned(), Value::Array(warnings));
}

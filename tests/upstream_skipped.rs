use hash::pipeline::enricher;
use serde_json::{Value, json};

#[test]
fn detects_skipped_records_correctly() {
    let skipped_record = json!({
        "path": "/data/file.txt",
        "version": "vacuum.v0",
        "_skipped": true,
        "_warnings": [{"tool": "vacuum", "code": "E_PERMISSION_DENIED"}]
    });

    let normal_record = json!({
        "path": "/data/file.txt",
        "version": "vacuum.v0",
        "size": 1024
    });

    let record_without_skipped = json!({
        "path": "/data/file.txt",
        "version": "vacuum.v0",
        "_skipped": false
    });

    assert!(enricher::is_skipped(skipped_record.as_object().unwrap()));
    assert!(!enricher::is_skipped(normal_record.as_object().unwrap()));
    assert!(!enricher::is_skipped(
        record_without_skipped.as_object().unwrap()
    ));
}

#[test]
fn process_skipped_record_preserves_warnings_and_skipped_flag() {
    let input = json!({
        "path": "/data/permission_denied.txt",
        "version": "vacuum.v0",
        "size": 1024,
        "mtime": "2025-12-31T12:00:00.000Z",
        "_skipped": true,
        "_warnings": [
            {
                "tool": "vacuum",
                "code": "E_PERMISSION_DENIED",
                "message": "Permission denied",
                "detail": {"path": "/data/permission_denied.txt"}
            }
        ],
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result = enricher::process_skipped_record(input);

    // Should update version to hash.v0
    assert_eq!(result["version"], "hash.v0");

    // Should set hash fields to null
    assert_eq!(result["bytes_hash"], Value::Null);
    assert_eq!(result["hash_algorithm"], Value::Null);

    // Should preserve _skipped flag
    assert_eq!(result["_skipped"], true);

    // Should preserve original _warnings unchanged
    let warnings = &result["_warnings"];
    assert!(warnings.is_array());
    let warning_array = warnings.as_array().unwrap();
    assert_eq!(warning_array.len(), 1);
    assert_eq!(warning_array[0]["tool"], "vacuum");
    assert_eq!(warning_array[0]["code"], "E_PERMISSION_DENIED");

    // Should update tool_versions to include hash
    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));

    // Should preserve all other fields
    assert_eq!(result["path"], "/data/permission_denied.txt");
    assert_eq!(result["size"], 1024);
    assert_eq!(result["mtime"], "2025-12-31T12:00:00.000Z");
}

#[test]
fn process_hashed_record_adds_hash_fields() {
    let input = json!({
        "path": "/data/normal.txt",
        "version": "vacuum.v0",
        "size": 2048,
        "mtime": "2025-12-31T12:00:00.000Z",
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let hash_value = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let result = enricher::process_hashed_record(input, hash_value.to_string(), "sha256");

    // Should update version to hash.v0
    assert_eq!(result["version"], "hash.v0");

    // Should set hash fields
    assert_eq!(result["bytes_hash"], hash_value);
    assert_eq!(result["hash_algorithm"], "sha256");

    // Should NOT have _skipped flag
    assert!(result.get("_skipped").is_none());

    // Should update tool_versions to include hash
    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));

    // Should preserve all other fields
    assert_eq!(result["path"], "/data/normal.txt");
    assert_eq!(result["size"], 2048);
    assert_eq!(result["mtime"], "2025-12-31T12:00:00.000Z");
}

#[test]
fn skipped_record_has_uniform_hash_schema() {
    let input = json!({
        "path": "/data/missing.txt",
        "version": "vacuum.v0",
        "_skipped": true,
        "_warnings": [{"tool": "vacuum", "code": "E_FILE_NOT_FOUND"}]
    });

    let result = enricher::process_skipped_record(input);

    // Both hash fields must be present and null for schema uniformity
    assert!(result.get("bytes_hash").is_some());
    assert_eq!(result["bytes_hash"], Value::Null);

    assert!(result.get("hash_algorithm").is_some());
    assert_eq!(result["hash_algorithm"], Value::Null);
}

#[test]
fn tool_versions_merge_preserves_upstream_versions() {
    let input = json!({
        "path": "/data/multi_tool.txt",
        "version": "fingerprint.v0",
        "_skipped": true,
        "tool_versions": {
            "vacuum": "0.1.0",
            "shape": "0.2.0",
            "fingerprint": "0.3.0"
        }
    });

    let result = enricher::process_skipped_record(input);

    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions.len(), 4); // 3 upstream + hash
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["shape"], "0.2.0");
    assert_eq!(tool_versions["fingerprint"], "0.3.0");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn handles_missing_tool_versions_gracefully() {
    let input = json!({
        "path": "/data/no_versions.txt",
        "version": "vacuum.v0",
        "_skipped": true
    });

    let result = enricher::process_skipped_record(input);

    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions.len(), 1); // only hash
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn multiple_warnings_are_preserved() {
    let input = json!({
        "path": "/data/multi_warnings.txt",
        "version": "vacuum.v0",
        "_skipped": true,
        "_warnings": [
            {"tool": "vacuum", "code": "E_LARGE_FILE"},
            {"tool": "shape", "code": "E_INVALID_CSV"}
        ]
    });

    let result = enricher::process_skipped_record(input);

    let warnings = result["_warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0]["tool"], "vacuum");
    assert_eq!(warnings[0]["code"], "E_LARGE_FILE");
    assert_eq!(warnings[1]["tool"], "shape");
    assert_eq!(warnings[1]["code"], "E_INVALID_CSV");
}

#[test]
fn false_skipped_value_is_treated_as_not_skipped() {
    let input = json!({
        "path": "/data/false_skip.txt",
        "version": "vacuum.v0",
        "_skipped": false
    });

    assert!(!enricher::is_skipped(input.as_object().unwrap()));
}

#[test]
fn non_boolean_skipped_value_is_treated_as_not_skipped() {
    let input = json!({
        "path": "/data/invalid_skip.txt",
        "version": "vacuum.v0",
        "_skipped": "true" // string instead of boolean
    });

    assert!(!enricher::is_skipped(input.as_object().unwrap()));
}

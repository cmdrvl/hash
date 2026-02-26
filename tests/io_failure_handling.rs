use hash::pipeline::enricher;
use serde_json::{Value, json};

#[test]
fn io_failed_record_is_marked_as_skipped() {
    let input = json!({
        "path": "/data/permission_denied.txt",
        "version": "vacuum.v0",
        "size": 1024,
        "mtime": "2025-12-31T12:00:00.000Z",
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result = enricher::process_io_failed_record(
        input,
        "/data/permission_denied.txt",
        "Permission denied",
    );

    // Should be marked as skipped
    assert_eq!(result["_skipped"], true);

    // Hash fields should be null
    assert_eq!(result["bytes_hash"], Value::Null);
    assert_eq!(result["hash_algorithm"], Value::Null);

    // Version should be updated
    assert_eq!(result["version"], "hash.v0");
}

#[test]
fn io_failed_record_appends_warning_with_correct_structure() {
    let input = json!({
        "path": "/data/missing.txt",
        "version": "vacuum.v0",
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result =
        enricher::process_io_failed_record(input, "/data/missing.txt", "No such file or directory");

    // Should have warnings array
    let warnings = result["_warnings"]
        .as_array()
        .expect("Should have warnings array");
    assert_eq!(warnings.len(), 1);

    let warning = &warnings[0];
    assert_eq!(warning["tool"], "hash");
    assert_eq!(warning["code"], "E_IO");
    assert_eq!(warning["message"], "Cannot read file");

    let detail = &warning["detail"];
    assert_eq!(detail["path"], "/data/missing.txt");
    assert_eq!(detail["error"], "No such file or directory");
}

#[test]
fn io_failed_record_preserves_existing_warnings() {
    let input = json!({
        "path": "/data/multi_issue.txt",
        "version": "vacuum.v0",
        "_warnings": [
            {
                "tool": "vacuum",
                "code": "E_LARGE_FILE",
                "message": "File is very large",
                "detail": {"size": 999999999}
            }
        ],
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result = enricher::process_io_failed_record(input, "/data/multi_issue.txt", "Read timeout");

    // Should have both warnings
    let warnings = result["_warnings"]
        .as_array()
        .expect("Should have warnings array");
    assert_eq!(warnings.len(), 2);

    // Original warning should be preserved
    assert_eq!(warnings[0]["tool"], "vacuum");
    assert_eq!(warnings[0]["code"], "E_LARGE_FILE");

    // New warning should be appended
    assert_eq!(warnings[1]["tool"], "hash");
    assert_eq!(warnings[1]["code"], "E_IO");
    assert_eq!(warnings[1]["detail"]["error"], "Read timeout");
}

#[test]
fn io_failed_record_updates_tool_versions() {
    let input = json!({
        "path": "/data/locked.txt",
        "version": "vacuum.v0",
        "tool_versions": {
            "vacuum": "0.1.0",
            "shape": "0.2.0"
        }
    });

    let result = enricher::process_io_failed_record(
        input,
        "/data/locked.txt",
        "Resource temporarily unavailable",
    );

    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["shape"], "0.2.0");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn io_failed_record_preserves_all_other_fields() {
    let input = json!({
        "path": "/data/complex.csv",
        "version": "vacuum.v0",
        "relative_path": "complex.csv",
        "root": "/data",
        "size": 8192,
        "mtime": "2025-12-31T12:00:00.000Z",
        "extension": ".csv",
        "mime_guess": "text/csv",
        "custom_field": "custom_value",
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result = enricher::process_io_failed_record(input, "/data/complex.csv", "Device not ready");

    // All original fields should be preserved
    assert_eq!(result["path"], "/data/complex.csv");
    assert_eq!(result["relative_path"], "complex.csv");
    assert_eq!(result["root"], "/data");
    assert_eq!(result["size"], 8192);
    assert_eq!(result["mtime"], "2025-12-31T12:00:00.000Z");
    assert_eq!(result["extension"], ".csv");
    assert_eq!(result["mime_guess"], "text/csv");
    assert_eq!(result["custom_field"], "custom_value");

    // Only version, hash fields, _skipped, _warnings, and tool_versions should be modified
    assert_eq!(result["version"], "hash.v0");
    assert_eq!(result["_skipped"], true);
    assert_eq!(result["bytes_hash"], Value::Null);
    assert_eq!(result["hash_algorithm"], Value::Null);
}

#[test]
fn io_failed_record_handles_missing_tool_versions() {
    let input = json!({
        "path": "/data/no_versions.txt",
        "version": "vacuum.v0"
    });

    let result = enricher::process_io_failed_record(
        input,
        "/data/no_versions.txt",
        "Operation not permitted",
    );

    let tool_versions = result["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions.len(), 1);
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn io_warning_has_expected_schema_shape() {
    let input = json!({
        "path": "/test/file.txt",
        "version": "vacuum.v0",
        "tool_versions": {}
    });

    let result =
        enricher::process_io_failed_record(input, "/test/file.txt", "Connection reset by peer");

    let warning = &result["_warnings"][0];

    // Verify warning has all required fields
    assert!(warning.get("tool").is_some());
    assert!(warning.get("code").is_some());
    assert!(warning.get("message").is_some());
    assert!(warning.get("detail").is_some());

    // Verify detail has path and error
    let detail = &warning["detail"];
    assert!(detail.get("path").is_some());
    assert!(detail.get("error").is_some());
}

#[test]
fn different_error_messages_are_captured() {
    let error_cases = [
        "Permission denied",
        "No such file or directory",
        "Device not ready",
        "Read-only file system",
        "Connection timeout",
    ];

    for (i, error_msg) in error_cases.iter().enumerate() {
        let input = json!({
            "path": format!("/data/test_{}.txt", i),
            "version": "vacuum.v0",
            "tool_versions": {}
        });

        let result =
            enricher::process_io_failed_record(input, &format!("/data/test_{}.txt", i), error_msg);

        let warning = &result["_warnings"][0];
        assert_eq!(warning["detail"]["error"], *error_msg);
        assert_eq!(warning["detail"]["path"], format!("/data/test_{}.txt", i));
    }
}

#[test]
fn non_object_record_is_returned_unchanged() {
    let input = Value::String("not an object".to_string());
    let result = enricher::process_io_failed_record(input.clone(), "/some/path", "some error");

    assert_eq!(result, input);
}

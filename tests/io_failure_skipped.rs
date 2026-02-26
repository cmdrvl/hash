use hash::pipeline::enricher::process_io_failed_record;
use serde_json::{Value, json};

#[test]
fn io_failed_record_marks_skipped_and_appends_warning() {
    let input = json!({
        "path": "/data/missing.csv",
        "version": "vacuum.v0",
        "size": 1024,
        "_warnings": [
            {
                "tool": "vacuum",
                "code": "E_METADATA",
                "message": "metadata warning",
                "detail": {"path": "/data/missing.csv"}
            }
        ],
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    let result = process_io_failed_record(input, "/data/missing.csv", "Permission denied");

    assert_eq!(result["version"], "hash.v0");
    assert_eq!(result["_skipped"], true);
    assert_eq!(result["bytes_hash"], Value::Null);
    assert_eq!(result["hash_algorithm"], Value::Null);
    assert_eq!(result["path"], "/data/missing.csv");
    assert_eq!(result["size"], 1024);

    let warnings = result["_warnings"]
        .as_array()
        .expect("_warnings should be an array");
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0]["tool"], "vacuum");

    let appended = warnings.last().expect("appended warning");
    assert_eq!(appended["tool"], "hash");
    assert_eq!(appended["code"], "E_IO");
    assert_eq!(appended["message"], "Cannot read file");
    assert_eq!(
        appended["detail"],
        json!({
            "path": "/data/missing.csv",
            "error": "Permission denied"
        })
    );

    let tool_versions = result["tool_versions"]
        .as_object()
        .expect("tool_versions should be object");
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn io_failed_record_creates_warning_array_when_missing() {
    let input = json!({
        "path": "/data/gone.csv",
        "version": "vacuum.v0"
    });

    let result = process_io_failed_record(input, "/data/gone.csv", "No such file or directory");

    let warnings = result["_warnings"]
        .as_array()
        .expect("_warnings should be created");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["tool"], "hash");
    assert_eq!(warnings[0]["code"], "E_IO");
    assert_eq!(
        warnings[0]["detail"],
        json!({
            "path": "/data/gone.csv",
            "error": "No such file or directory"
        })
    );
}

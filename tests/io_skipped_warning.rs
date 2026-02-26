use hash::pipeline::enricher::process_file_io_error;
use serde_json::{Value, json};

#[test]
fn file_io_error_marks_record_as_skipped_with_eio_warning() {
    let input = json!({
        "path": "/data/denied.csv",
        "version": "vacuum.v0",
        "tool_versions": {"vacuum": "0.1.0"}
    });

    let output = process_file_io_error(input, "Permission denied");

    assert_eq!(output["version"], "hash.v0");
    assert_eq!(output["_skipped"], true);
    assert_eq!(output["bytes_hash"], Value::Null);
    assert_eq!(output["hash_algorithm"], Value::Null);

    let warnings = output["_warnings"]
        .as_array()
        .expect("_warnings should be an array");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["tool"], "hash");
    assert_eq!(warnings[0]["code"], "E_IO");
    assert_eq!(warnings[0]["message"], "Cannot read file");
    assert_eq!(warnings[0]["detail"]["path"], "/data/denied.csv");
    assert_eq!(warnings[0]["detail"]["error"], "Permission denied");

    let versions = output["tool_versions"]
        .as_object()
        .expect("tool_versions should be an object");
    assert_eq!(versions["vacuum"], "0.1.0");
    assert_eq!(versions["hash"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn file_io_error_appends_warning_to_existing_warning_array() {
    let input = json!({
        "path": "/data/gone.csv",
        "version": "vacuum.v0",
        "_warnings": [
            {
                "tool": "vacuum",
                "code": "E_STALE",
                "message": "stale metadata"
            }
        ]
    });

    let output = process_file_io_error(input, "No such file or directory");
    let warnings = output["_warnings"]
        .as_array()
        .expect("_warnings should be an array");

    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0]["tool"], "vacuum");
    assert_eq!(warnings[1]["tool"], "hash");
    assert_eq!(warnings[1]["code"], "E_IO");
    assert_eq!(warnings[1]["detail"]["path"], "/data/gone.csv");
}

use hash::pipeline::enricher::apply_upstream_skipped_passthrough;
use serde_json::{Map, Value, json};

#[test]
fn upstream_skipped_passthrough_preserves_warnings_and_sets_hash_fields_null() {
    let warnings = json!([{
        "tool": "vacuum",
        "code": "E_IO",
        "message": "could not read metadata",
        "detail": {"path": "/tmp/missing.csv"}
    }]);

    let mut record = Map::new();
    record.insert("version".to_owned(), json!("vacuum.v0"));
    record.insert("path".to_owned(), json!("/tmp/missing.csv"));
    record.insert("_skipped".to_owned(), json!(true));
    record.insert("_warnings".to_owned(), warnings.clone());
    record.insert("tool_versions".to_owned(), json!({"vacuum": "0.1.0"}));
    record.insert("custom_field".to_owned(), json!("keep-me"));

    apply_upstream_skipped_passthrough(&mut record);

    assert_eq!(record.get("version"), Some(&json!("hash.v0")));
    assert_eq!(record.get("bytes_hash"), Some(&Value::Null));
    assert_eq!(record.get("hash_algorithm"), Some(&Value::Null));
    assert_eq!(record.get("_skipped"), Some(&json!(true)));
    assert_eq!(record.get("_warnings"), Some(&warnings));
    assert_eq!(record.get("custom_field"), Some(&json!("keep-me")));

    let versions = record
        .get("tool_versions")
        .and_then(Value::as_object)
        .expect("tool_versions should be object");
    assert_eq!(versions.get("vacuum"), Some(&json!("0.1.0")));
    assert!(versions.get("hash").is_some());
}

#[test]
fn passthrough_creates_tool_versions_when_missing_or_invalid() {
    let mut missing_versions = Map::new();
    missing_versions.insert("version".to_owned(), json!("vacuum.v0"));
    apply_upstream_skipped_passthrough(&mut missing_versions);
    assert!(
        missing_versions
            .get("tool_versions")
            .and_then(Value::as_object)
            .and_then(|versions| versions.get("hash"))
            .is_some()
    );

    let mut invalid_versions = Map::new();
    invalid_versions.insert("version".to_owned(), json!("vacuum.v0"));
    invalid_versions.insert("tool_versions".to_owned(), json!("unexpected"));
    apply_upstream_skipped_passthrough(&mut invalid_versions);
    assert!(
        invalid_versions
            .get("tool_versions")
            .and_then(Value::as_object)
            .and_then(|versions| versions.get("hash"))
            .is_some()
    );
}

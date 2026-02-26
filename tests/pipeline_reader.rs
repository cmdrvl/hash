use hash::pipeline::reader::parse_json_line;
use serde_json::json;

#[test]
fn parse_json_line_accepts_valid_record_with_required_fields() {
    let parsed = parse_json_line(
        r#"{"path":"/tmp/input.csv","version":"vacuum.v0","size":12}"#,
        3,
    )
    .expect("valid record should parse");

    assert_eq!(parsed.line_number, 3);
    assert_eq!(parsed.record["path"], "/tmp/input.csv");
    assert_eq!(parsed.record["version"], "vacuum.v0");
    assert_eq!(parsed.record["size"], 12);
}

#[test]
fn parse_json_line_maps_invalid_json_to_bad_input_refusal() {
    let refusal = parse_json_line(r#"{"path": "/tmp/a.csv""#, 9).expect_err("invalid json");

    assert_eq!(refusal.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        refusal.refusal.message,
        "Input is not valid JSONL or missing required fields"
    );
    assert_eq!(refusal.refusal.detail["line"], 9);
    assert!(refusal.refusal.detail["error"].as_str().is_some());
}

#[test]
fn parse_json_line_requires_path_and_version_fields() {
    let missing_path =
        parse_json_line(r#"{"version":"vacuum.v0"}"#, 11).expect_err("path is required");
    assert_eq!(missing_path.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        missing_path.refusal.detail,
        json!({"line": 11, "missing_field": "path"})
    );

    let missing_version =
        parse_json_line(r#"{"path":"/tmp/input.csv"}"#, 12).expect_err("version is required");
    assert_eq!(missing_version.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        missing_version.refusal.detail,
        json!({"line": 12, "missing_field": "version"})
    );
}

#[test]
fn parse_json_line_rejects_non_object_records() {
    let refusal = parse_json_line(r#"["not","an","object"]"#, 5)
        .expect_err("top-level arrays are invalid record shapes");

    assert_eq!(refusal.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        refusal.refusal.detail,
        json!({ "line": 5, "error": "record must be a JSON object" })
    );
}

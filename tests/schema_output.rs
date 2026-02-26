use serde_json::Value;
use std::fs;
use std::process::Command;

#[test]
fn schema_flag_emits_canonical_schema_artifact() {
    let expected_schema = fs::read_to_string("schema/hash.v0.schema.json")
        .expect("schema/hash.v0.schema.json should be present");
    let expected_value: Value =
        serde_json::from_str(&expected_schema).expect("schema artifact must be valid JSON");

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--schema")
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());

    let output_text = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let actual_value: Value =
        serde_json::from_str(&output_text).expect("--schema output must be valid JSON");

    assert_eq!(actual_value, expected_value);
    assert_eq!(
        output_text.trim_end_matches('\n'),
        expected_schema.trim_end_matches('\n')
    );
}

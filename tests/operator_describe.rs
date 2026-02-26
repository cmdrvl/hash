use serde_json::Value;
use std::fs;
use std::process::Command;

#[test]
fn describe_emits_canonical_operator_manifest() {
    let expected_manifest =
        fs::read_to_string("operator.json").expect("operator.json should be present");
    let expected_value: Value =
        serde_json::from_str(&expected_manifest).expect("operator.json must be valid JSON");

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--describe")
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());

    let output_text = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let actual_value: Value =
        serde_json::from_str(&output_text).expect("--describe output must be valid JSON");

    assert_eq!(actual_value, expected_value);
    assert_eq!(
        output_text.trim_end_matches('\n'),
        expected_manifest.trim_end_matches('\n')
    );
}

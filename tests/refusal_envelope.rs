use hash::refusal::{RefusalCode, RefusalEnvelope};
use serde_json::{Value, json};

#[test]
fn creates_envelope_with_correct_version_and_outcome() {
    let envelope = RefusalEnvelope::new(
        RefusalCode::BadInput,
        "test message",
        json!({"test": "detail"}),
    );

    assert_eq!(envelope.version, "hash.v0");
    assert_eq!(envelope.outcome, "REFUSAL");
    assert_eq!(envelope.refusal.code, "E_BAD_INPUT");
    assert_eq!(envelope.refusal.message, "test message");
}

#[test]
fn bad_input_parse_error_creates_correct_detail() {
    let envelope = RefusalEnvelope::bad_input_parse_error(42, "expected value at line 1 column 1");

    assert_eq!(envelope.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        envelope.refusal.message,
        "Input is not valid JSONL or missing required fields"
    );

    let expected_detail = json!({
        "line": 42,
        "error": "expected value at line 1 column 1"
    });
    assert_eq!(envelope.refusal.detail, expected_detail);
    assert_eq!(envelope.refusal.next_command, None);
}

#[test]
fn bad_input_missing_field_creates_correct_detail() {
    let envelope = RefusalEnvelope::bad_input_missing_field(1, "path");

    assert_eq!(envelope.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        envelope.refusal.message,
        "Input is not valid JSONL or missing required fields"
    );

    let expected_detail = json!({
        "line": 1,
        "missing_field": "path"
    });
    assert_eq!(envelope.refusal.detail, expected_detail);
}

#[test]
fn io_error_creates_correct_detail() {
    let envelope = RefusalEnvelope::io_error("Broken pipe");

    assert_eq!(envelope.refusal.code, "E_IO");
    assert_eq!(envelope.refusal.message, "Cannot read input/output stream");

    let expected_detail = json!({
        "error": "Broken pipe"
    });
    assert_eq!(envelope.refusal.detail, expected_detail);
}

#[test]
fn from_code_uses_default_message() {
    let envelope = RefusalEnvelope::from_code(RefusalCode::Io, json!({"error": "custom error"}));

    assert_eq!(envelope.refusal.code, "E_IO");
    assert_eq!(envelope.refusal.message, "Cannot read input/output stream");
}

#[test]
fn with_next_command_sets_next_command() {
    let envelope =
        RefusalEnvelope::io_error("test error").with_next_command("Check stdin availability");

    assert_eq!(
        envelope.refusal.next_command,
        Some("Check stdin availability".to_string())
    );
}

#[test]
fn to_value_serializes_correctly() {
    let envelope = RefusalEnvelope::bad_input_parse_error(5, "invalid json");
    let value = envelope.to_value().expect("Should serialize successfully");

    let expected = json!({
        "version": "hash.v0",
        "outcome": "REFUSAL",
        "refusal": {
            "code": "E_BAD_INPUT",
            "message": "Input is not valid JSONL or missing required fields",
            "detail": {
                "line": 5,
                "error": "invalid json"
            },
            "next_command": null
        }
    });

    assert_eq!(value, expected);
}

#[test]
fn envelope_serializes_to_json_string() {
    let envelope = RefusalEnvelope::io_error("Connection refused");
    let json_string = serde_json::to_string(&envelope).expect("Should serialize to JSON");

    // Verify it can be parsed back
    let parsed: Value = serde_json::from_str(&json_string).expect("Should parse back");
    assert_eq!(parsed["version"], "hash.v0");
    assert_eq!(parsed["outcome"], "REFUSAL");
    assert_eq!(parsed["refusal"]["code"], "E_IO");
}

#[test]
fn refusal_codes_have_correct_string_representations() {
    assert_eq!(RefusalCode::BadInput.as_str(), "E_BAD_INPUT");
    assert_eq!(RefusalCode::Io.as_str(), "E_IO");

    // Test Display trait
    assert_eq!(format!("{}", RefusalCode::BadInput), "E_BAD_INPUT");
    assert_eq!(format!("{}", RefusalCode::Io), "E_IO");
}

#[test]
fn refusal_codes_have_appropriate_default_messages() {
    assert_eq!(
        RefusalCode::BadInput.default_message(),
        "Input is not valid JSONL or missing required fields"
    );
    assert_eq!(
        RefusalCode::Io.default_message(),
        "Cannot read input/output stream"
    );
}

#[test]
fn different_envelopes_with_same_content_are_equal() {
    let envelope1 = RefusalEnvelope::bad_input_missing_field(10, "path");
    let envelope2 = RefusalEnvelope::bad_input_missing_field(10, "path");

    assert_eq!(envelope1, envelope2);
}

#[test]
fn envelope_serializes_to_valid_json() {
    let original = RefusalEnvelope::bad_input_parse_error(123, "syntax error");
    let json = serde_json::to_string(&original).expect("Should serialize");

    // Verify the JSON is valid and contains expected structure
    let parsed: Value = serde_json::from_str(&json).expect("Should parse as valid JSON");
    assert_eq!(parsed["version"], "hash.v0");
    assert_eq!(parsed["outcome"], "REFUSAL");
    assert_eq!(parsed["refusal"]["code"], "E_BAD_INPUT");
    assert_eq!(parsed["refusal"]["detail"]["line"], 123);
    assert_eq!(parsed["refusal"]["detail"]["error"], "syntax error");
}

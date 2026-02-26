use hash::refusal::{RefusalCode, RefusalEnvelope};
use serde_json::json;

#[test]
fn refusal_codes_map_to_contract_values() {
    assert_eq!(RefusalCode::BadInput.as_str(), "E_BAD_INPUT");
    assert_eq!(RefusalCode::Io.as_str(), "E_IO");
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
fn bad_input_parse_error_shape_is_stable() {
    let refusal = RefusalEnvelope::bad_input_parse_error(7, "expected value");
    assert_eq!(refusal.version, "hash.v0");
    assert_eq!(refusal.outcome, "REFUSAL");
    assert_eq!(refusal.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        refusal.refusal.message,
        "Input is not valid JSONL or missing required fields"
    );
    assert_eq!(
        refusal.refusal.detail,
        json!({"line": 7, "error": "expected value"})
    );
    assert!(refusal.refusal.next_command.is_none());
}

#[test]
fn bad_input_missing_field_shape_is_stable() {
    let refusal = RefusalEnvelope::bad_input_missing_field(3, "path");
    assert_eq!(refusal.refusal.code, "E_BAD_INPUT");
    assert_eq!(
        refusal.refusal.detail,
        json!({"line": 3, "missing_field": "path"})
    );
}

#[test]
fn io_error_shape_and_next_command_are_supported() {
    let refusal = RefusalEnvelope::io_error("Broken pipe").with_next_command("check stdin");
    assert_eq!(refusal.refusal.code, "E_IO");
    assert_eq!(refusal.refusal.message, "Cannot read input/output stream");
    assert_eq!(refusal.refusal.detail, json!({"error": "Broken pipe"}));
    assert_eq!(refusal.refusal.next_command.as_deref(), Some("check stdin"));
}

#[test]
fn to_value_serializes_expected_envelope() {
    let refusal = RefusalEnvelope::io_error("Permission denied");
    let value = refusal.to_value().expect("serialization succeeds");
    assert_eq!(
        value,
        json!({
            "version": "hash.v0",
            "outcome": "REFUSAL",
            "refusal": {
                "code": "E_IO",
                "message": "Cannot read input/output stream",
                "detail": { "error": "Permission denied" },
                "next_command": null
            }
        })
    );
}

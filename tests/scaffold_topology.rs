use hash::cli::{Algorithm, Outcome, exit_code};
use hash::output::jsonl::write_json_line;
use hash::pipeline::parallel::OrderedResults;
use hash::pipeline::reader::parse_json_line;
use hash::refusal::{RefusalCode, RefusalEnvelope};
use hash::witness::query::{WitnessQuery, filter_records};
use hash::witness::record::WitnessRecord;
use serde_json::json;
use std::str::FromStr;

#[test]
fn scaffold_outcome_exit_codes_match_contract() {
    assert_eq!(exit_code(Outcome::AllHashed), 0);
    assert_eq!(exit_code(Outcome::Partial), 1);
    assert_eq!(exit_code(Outcome::Refusal), 2);
}

#[test]
fn scaffold_algorithm_parser_is_case_insensitive() {
    assert_eq!(
        Algorithm::from_str("SHA256").expect("sha256 should parse"),
        Algorithm::Sha256
    );
    assert_eq!(
        Algorithm::from_str("blake3").expect("blake3 should parse"),
        Algorithm::Blake3
    );
}

#[test]
fn scaffold_ordered_results_emits_in_input_order() {
    let mut ordered = OrderedResults::new();
    assert!(ordered.push(1, "second").is_empty());
    assert_eq!(ordered.push(0, "first"), vec!["first", "second"]);
}

#[test]
fn scaffold_reader_output_refusal_and_witness_stubs_are_wired() {
    let parsed = parse_json_line(r#"{"path":"/tmp/file.csv"}"#, 7).expect("valid json");
    assert_eq!(parsed.line_number, 7);
    assert_eq!(parsed.record["path"], "/tmp/file.csv");

    let mut output = Vec::new();
    write_json_line(&mut output, &json!({"ok": true})).expect("json line write");
    assert_eq!(output, b"{\"ok\":true}\n");

    let refusal = RefusalEnvelope::new(RefusalCode::BadInput, "invalid line", json!({"line": 7}));
    assert_eq!(refusal.refusal.code, "E_BAD_INPUT");

    let records = vec![
        WitnessRecord::new("hash", "ALL_HASHED", 0),
        WitnessRecord::new("hash", "REFUSAL", 2),
    ];
    let filtered = filter_records(
        &records,
        &WitnessQuery {
            tool: Some("hash".to_owned()),
            outcome: Some("REFUSAL".to_owned()),
        },
    );
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].exit_code, 2);
}

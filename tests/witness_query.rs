use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    path.push(format!(
        "hash-witness-query-{}-{suffix}-{nanos}.jsonl",
        std::process::id()
    ));
    path
}

fn write_witness_records(path: &Path, records: &[Value]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create witness parent");
    }

    let mut file = fs::File::create(path).expect("create witness file");
    for record in records {
        let line = serde_json::to_string(record).expect("serialize witness line");
        writeln!(file, "{line}").expect("write witness line");
    }
}

fn run_hash_with_witness(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hash"))
        .env("EPISTEMIC_WITNESS", path)
        .args(args)
        .output()
        .expect("hash binary should run")
}

fn sample_records() -> Vec<Value> {
    vec![
        json!({
            "version": "witness.v0",
            "tool": "hash",
            "outcome": "ALL_HASHED",
            "exit_code": 0,
            "output_hash": "blake3:aaa111",
            "ts": "2026-01-01T12:00:00Z",
            "params": {}
        }),
        json!({
            "version": "witness.v0",
            "tool": "lock",
            "outcome": "REFUSAL",
            "exit_code": 2,
            "output_hash": "blake3:bbb222",
            "ts": "2026-01-02T12:00:00Z",
            "params": {}
        }),
        json!({
            "version": "witness.v0",
            "tool": "hash",
            "outcome": "PARTIAL",
            "exit_code": 1,
            "output_hash": "blake3:ccc333",
            "ts": "2026-01-03T12:00:00Z",
            "params": {}
        }),
    ]
}

#[test]
fn query_applies_tool_outcome_hash_and_limit_filters() {
    let witness_path = unique_path("query-filters");
    write_witness_records(&witness_path, &sample_records());

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "witness",
            "query",
            "--tool",
            "hash",
            "--outcome",
            "PARTIAL",
            "--input-hash",
            "ccc",
            "--limit",
            "1",
            "--json",
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let parsed: Value = serde_json::from_str(&stdout).expect("query json output");
    let rows = parsed.as_array().expect("array output");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["tool"], "hash");
    assert_eq!(rows[0]["outcome"], "PARTIAL");

    let _ = fs::remove_file(witness_path);
}

#[test]
fn query_applies_since_and_until_bounds() {
    let witness_path = unique_path("query-time");
    write_witness_records(&witness_path, &sample_records());

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "witness",
            "query",
            "--since",
            "2026-01-02T00:00:00Z",
            "--until",
            "2026-01-02T23:59:59Z",
            "--json",
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let parsed: Value = serde_json::from_str(&stdout).expect("query json output");
    let rows = parsed.as_array().expect("array output");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["tool"], "lock");
    assert_eq!(rows[0]["outcome"], "REFUSAL");

    let _ = fs::remove_file(witness_path);
}

#[test]
fn last_returns_null_and_exit_one_when_ledger_is_empty() {
    let witness_path = unique_path("last-empty");
    let _ = fs::remove_file(&witness_path);

    let output = run_hash_with_witness(&witness_path, &["witness", "last", "--json"]);
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert_eq!(stdout.trim(), "null");
}

#[test]
fn count_outputs_json_and_respects_match_exit_codes() {
    let witness_path = unique_path("count-json");
    write_witness_records(&witness_path, &sample_records());

    let matching = run_hash_with_witness(
        &witness_path,
        &["witness", "count", "--tool", "hash", "--json"],
    );
    assert_eq!(matching.status.code(), Some(0));
    let matching_stdout = String::from_utf8(matching.stdout).expect("stdout utf8");
    assert_eq!(
        serde_json::from_str::<Value>(&matching_stdout).expect("count json"),
        json!({ "count": 2 })
    );

    let missing = run_hash_with_witness(
        &witness_path,
        &["witness", "count", "--tool", "missing", "--json"],
    );
    assert_eq!(missing.status.code(), Some(1));
    let missing_stdout = String::from_utf8(missing.stdout).expect("stdout utf8");
    assert_eq!(
        serde_json::from_str::<Value>(&missing_stdout).expect("count json"),
        json!({ "count": 0 })
    );

    let _ = fs::remove_file(witness_path);
}

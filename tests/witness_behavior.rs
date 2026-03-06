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
        "hash-witness-behavior-{}-{suffix}-{nanos}.jsonl",
        std::process::id()
    ));
    path
}

fn run_hash_with_witness(path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hashbytes"))
        .env("EPISTEMIC_WITNESS", path)
        .args(args)
        .output()
        .expect("hash binary should run")
}

fn write_witness_lines(path: &Path, lines: &[&str]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create witness parent");
    }
    let mut file = fs::File::create(path).expect("create witness file");
    for line in lines {
        writeln!(file, "{line}").expect("write witness line");
    }
}

fn witness_line(
    tool: &str,
    outcome: &str,
    exit_code: u8,
    output_hash: &str,
    input_hash: Option<&str>,
    ts: &str,
) -> String {
    serde_json::to_string(&json!({
        "id": format!("blake3:{tool}-{outcome}-{ts}"),
        "tool": tool,
        "version": "0.0.0-test",
        "binary_hash": "blake3:test-binary",
        "outcome": outcome,
        "exit_code": exit_code,
        "inputs": [{
            "path": "stdin",
            "hash": input_hash,
            "bytes": null
        }],
        "params": {},
        "output_hash": output_hash,
        "prev": null,
        "ts": ts,
    }))
    .expect("serialize witness line")
}

#[test]
fn default_runs_append_multiple_records_to_same_ledger() {
    let witness_path = unique_path("append-chaining");

    let first = run_hash_with_witness(&witness_path, &[]);
    assert_eq!(first.status.code(), Some(0));

    let second = run_hash_with_witness(&witness_path, &[]);
    assert_eq!(second.status.code(), Some(0));

    let contents = fs::read_to_string(&witness_path).expect("witness file should exist");
    let rows: Vec<Value> = contents
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid witness json"))
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(rows[0]["tool"], "hash");
    assert!(rows[0]["prev"].is_null());
    assert_eq!(rows[1]["tool"], "hash");
    assert_eq!(rows[1]["prev"], rows[0]["id"]);
    for row in rows {
        assert_eq!(row["outcome"], "ALL_HASHED");
        assert_eq!(row["exit_code"], 0);
        assert!(
            row["output_hash"]
                .as_str()
                .is_some_and(|value| value.starts_with("blake3:"))
        );
        assert!(
            row["binary_hash"]
                .as_str()
                .is_some_and(|value| value.starts_with("blake3:"))
        );
    }

    let _ = fs::remove_file(witness_path);
}

#[test]
fn no_witness_keeps_existing_ledger_unchanged() {
    let witness_path = unique_path("no-witness-existing");
    let seed = witness_line(
        "hash",
        "PARTIAL",
        1,
        "blake3:seed",
        Some("blake3:input-seed"),
        "2026-01-01T00:00:00Z",
    );
    write_witness_lines(&witness_path, &[&seed]);

    let output = run_hash_with_witness(&witness_path, &["--no-witness"]);
    assert_eq!(output.status.code(), Some(0));

    let contents = fs::read_to_string(&witness_path).expect("witness file should still exist");
    assert_eq!(contents.lines().count(), 1);

    let _ = fs::remove_file(witness_path);
}

#[test]
fn witness_records_normalized_jobs_value() {
    let witness_path = unique_path("normalized-jobs");

    let output = run_hash_with_witness(&witness_path, &["--jobs", "0"]);
    assert_eq!(output.status.code(), Some(0));

    let contents = fs::read_to_string(&witness_path).expect("witness file should exist");
    let last_row = contents
        .lines()
        .last()
        .expect("witness file should contain one record");
    let parsed: Value = serde_json::from_str(last_row).expect("valid witness json");
    assert_eq!(parsed["tool"], "hash");
    assert_eq!(parsed["params"]["jobs"], 1);

    let _ = fs::remove_file(witness_path);
}

#[test]
fn query_skips_malformed_lines_and_applies_filters() {
    let witness_path = unique_path("query-malformed");
    let good_match = witness_line(
        "hash",
        "PARTIAL",
        1,
        "blake3:result-123",
        Some("blake3:input-match-123"),
        "2026-01-03T12:00:00Z",
    );
    let good_non_match = witness_line(
        "lock",
        "REFUSAL",
        2,
        "blake3:other-456",
        Some("blake3:input-other-456"),
        "2026-01-04T12:00:00Z",
    );
    write_witness_lines(&witness_path, &["{bad-json", &good_match, &good_non_match]);

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
            "input-match",
            "--json",
        ],
    );

    assert_eq!(output.status.code(), Some(0));
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("query json output");
    let rows = parsed.as_array().expect("rows should be array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["tool"], "hash");
    assert_eq!(rows[0]["outcome"], "PARTIAL");
    assert_eq!(rows[0]["output_hash"], "blake3:result-123");

    let _ = fs::remove_file(witness_path);
}

#[test]
fn last_returns_most_recent_record_when_ledger_has_entries() {
    let witness_path = unique_path("last-non-empty");
    let older = witness_line(
        "hash",
        "ALL_HASHED",
        0,
        "blake3:older",
        Some("blake3:input-older"),
        "2026-01-01T00:00:00Z",
    );
    let newer = witness_line(
        "lock",
        "REFUSAL",
        2,
        "blake3:newer",
        Some("blake3:input-newer"),
        "2026-01-02T00:00:00Z",
    );
    write_witness_lines(&witness_path, &[&older, &newer]);

    let output = run_hash_with_witness(&witness_path, &["witness", "last", "--json"]);

    assert_eq!(output.status.code(), Some(0));
    let parsed: Value = serde_json::from_slice(&output.stdout).expect("last json output");
    assert_eq!(parsed["tool"], "lock");
    assert_eq!(parsed["outcome"], "REFUSAL");
    assert_eq!(parsed["exit_code"], 2);
    assert_eq!(parsed["output_hash"], "blake3:newer");

    let _ = fs::remove_file(witness_path);
}

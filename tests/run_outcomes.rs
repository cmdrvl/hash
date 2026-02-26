use hash::cli::{Cli, Command, WitnessAction};
use hash::run_with_cli;
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn base_cli() -> Cli {
    Cli {
        command: None,
        input: None,
        algorithm: "sha256".to_string(),
        jobs: None,
        no_witness: false,
        progress: false,
        describe: false,
        schema: false,
    }
}

fn unique_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    path.push(format!(
        "hash-run-outcomes-{}-{suffix}-{nanos}.jsonl",
        std::process::id()
    ));
    path
}

fn write_jsonl(path: &Path, records: &[Value]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create manifest parent");
    }

    let mut file = fs::File::create(path).expect("create manifest file");
    for record in records {
        let line = serde_json::to_string(record).expect("serialize manifest line");
        writeln!(file, "{line}").expect("write manifest line");
    }
}

fn run_hash_with_witness(witness_path: &Path, args: &[&str]) -> Output {
    ProcessCommand::new(env!("CARGO_BIN_EXE_hash"))
        .env("EPISTEMIC_WITNESS", witness_path)
        .args(args)
        .output()
        .expect("hash binary should run")
}

fn parse_jsonl(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8(stdout.to_vec())
        .expect("stdout utf8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .collect()
}

#[test]
fn describe_short_circuits_before_main_workflow() {
    let mut cli = base_cli();
    cli.describe = true;
    cli.algorithm = "invalid".to_string();
    assert_eq!(run_with_cli(cli), 0);
}

#[test]
fn schema_short_circuits_before_main_workflow() {
    let mut cli = base_cli();
    cli.schema = true;
    cli.algorithm = "invalid".to_string();
    assert_eq!(run_with_cli(cli), 0);
}

#[test]
fn witness_subcommands_route_to_witness_handler_exit_codes() {
    let mut cli = base_cli();
    cli.command = Some(Command::Witness {
        action: WitnessAction::Query {
            tool: Some("__no_such_tool__".to_string()),
            since: Some("2999-01-01T00:00:00Z".to_string()),
            until: None,
            outcome: None,
            input_hash: None,
            limit: None,
            json: true,
        },
    });

    assert_eq!(run_with_cli(cli), 1);
}

#[test]
fn invalid_algorithm_returns_refusal_exit_code() {
    let mut cli = base_cli();
    cli.algorithm = "md5".to_string();
    assert_eq!(run_with_cli(cli), 2);
}

#[test]
fn default_main_path_returns_all_hashed_exit_code() {
    assert_eq!(run_with_cli(base_cli()), 0);
}

#[test]
fn binary_all_hashed_returns_exit_zero_and_hash_fields() {
    let data_path = unique_path("all-hashed-data");
    let manifest_path = unique_path("all-hashed-manifest");
    let witness_path = unique_path("all-hashed-witness");

    fs::write(&data_path, b"hash all hashed fixture").expect("write fixture file");
    write_jsonl(
        &manifest_path,
        &[json!({
            "version": "vacuum.v0",
            "path": data_path.to_string_lossy()
        })],
    );

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "--no-witness",
            manifest_path.to_str().expect("manifest path utf8"),
        ],
    );
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["version"], "hash.v0");
    assert_eq!(rows[0]["hash_algorithm"], "sha256");
    assert!(
        rows[0]["bytes_hash"]
            .as_str()
            .expect("bytes_hash should be string")
            .starts_with("sha256:")
    );

    let _ = fs::remove_file(data_path);
    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

#[test]
fn binary_partial_returns_exit_one_for_unreadable_paths() {
    let missing_path = unique_path("missing-data");
    let manifest_path = unique_path("partial-manifest");
    let witness_path = unique_path("partial-witness");
    let _ = fs::remove_file(&missing_path);

    write_jsonl(
        &manifest_path,
        &[json!({
            "version": "vacuum.v0",
            "path": missing_path.to_string_lossy()
        })],
    );

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "--no-witness",
            manifest_path.to_str().expect("manifest path utf8"),
        ],
    );
    assert_eq!(output.status.code(), Some(1));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["_skipped"], true);
    assert_eq!(rows[0]["bytes_hash"], Value::Null);
    assert_eq!(rows[0]["hash_algorithm"], Value::Null);
    assert_eq!(rows[0]["_warnings"][0]["tool"], "hash");
    assert_eq!(rows[0]["_warnings"][0]["code"], "E_IO");

    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

#[test]
fn binary_partial_returns_exit_one_for_upstream_skipped_records() {
    let missing_path = unique_path("upstream-skipped-data");
    let manifest_path = unique_path("upstream-skipped-manifest");
    let witness_path = unique_path("upstream-skipped-witness");

    write_jsonl(
        &manifest_path,
        &[json!({
            "version": "vacuum.v0",
            "path": missing_path.to_string_lossy(),
            "_skipped": true,
            "_warnings": [
                {
                    "tool": "vacuum",
                    "code": "E_UPSTREAM",
                    "message": "upstream skipped"
                }
            ]
        })],
    );

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "--no-witness",
            manifest_path.to_str().expect("manifest path utf8"),
        ],
    );
    assert_eq!(output.status.code(), Some(1));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["_skipped"], true);
    assert_eq!(rows[0]["bytes_hash"], Value::Null);
    assert_eq!(rows[0]["hash_algorithm"], Value::Null);
    assert_eq!(rows[0]["_warnings"][0]["tool"], "vacuum");
    assert_eq!(rows[0]["_warnings"][0]["code"], "E_UPSTREAM");
    assert_eq!(
        rows[0]["_warnings"]
            .as_array()
            .expect("warnings should be an array")
            .len(),
        1
    );

    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

#[test]
fn binary_refusal_returns_exit_two_and_envelope_for_bad_jsonl() {
    let manifest_path = unique_path("bad-jsonl-manifest");
    let witness_path = unique_path("bad-jsonl-witness");

    fs::write(&manifest_path, b"not-json\n").expect("write invalid manifest");

    let output = run_hash_with_witness(
        &witness_path,
        &[
            "--no-witness",
            manifest_path.to_str().expect("manifest path utf8"),
        ],
    );
    assert_eq!(output.status.code(), Some(2));

    let refusal: Value = serde_json::from_slice(&output.stdout).expect("refusal envelope json");
    assert_eq!(refusal["version"], "hash.v0");
    assert_eq!(refusal["outcome"], "REFUSAL");
    assert_eq!(refusal["refusal"]["code"], "E_BAD_INPUT");

    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

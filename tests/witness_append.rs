use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    path.push(format!(
        "hash-witness-test-{}-{suffix}-{nanos}",
        std::process::id()
    ));
    path
}

#[test]
fn appends_witness_record_by_default() {
    let witness_path = unique_path("default").with_extension("jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());

    let contents = fs::read_to_string(&witness_path).expect("witness file should be written");
    let line = contents
        .lines()
        .last()
        .expect("witness file should contain one line");
    let witness: Value = serde_json::from_str(line).expect("witness line should be valid json");

    assert_eq!(witness["version"], "witness.v0");
    assert_eq!(witness["tool"], "hash");
    assert_eq!(witness["outcome"], "ALL_HASHED");
    assert_eq!(witness["exit_code"], 0);
    assert!(
        witness["output_hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("blake3:"))
    );

    let _ = fs::remove_file(witness_path);
}

#[test]
fn no_witness_flag_skips_append() {
    let witness_path = unique_path("disabled").with_extension("jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--no-witness")
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());
    assert!(!witness_path.exists());
}

#[test]
fn witness_append_failure_does_not_change_exit_code() {
    let witness_dir = unique_path("dir-target");
    fs::create_dir_all(&witness_dir).expect("dir target should be created");

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .env("EPISTEMIC_WITNESS", &witness_dir)
        .output()
        .expect("hash binary should run");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(witness_dir);
}

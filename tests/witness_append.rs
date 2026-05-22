use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod support;

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

fn canonical_witness_path(home: &Path) -> PathBuf {
    home.join(".cmdrvl")
        .join("state")
        .join("witness")
        .join("witness.jsonl")
}

#[test]
fn appends_witness_record_by_default() {
    let home = unique_path("default-home");
    fs::create_dir_all(&home).expect("temporary home should be created");
    let witness_path = canonical_witness_path(&home);

    let output = support::hashbytes_command("witness-append-default")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env_remove("EPISTEMIC_WITNESS")
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());

    let contents = fs::read_to_string(&witness_path).expect("witness file should be written");
    let line = contents
        .lines()
        .last()
        .expect("witness file should contain one line");
    let witness: Value = serde_json::from_str(line).expect("witness line should be valid json");

    assert!(
        witness["id"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
    assert_eq!(witness["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(witness["tool"], "hash");
    assert!(
        witness["binary_hash"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );
    assert_eq!(witness["outcome"], "ALL_HASHED");
    assert_eq!(witness["exit_code"], 0);
    assert_eq!(witness["inputs"][0]["path"], "stdin");
    assert!(witness["inputs"][0]["hash"].is_null());
    assert!(witness["inputs"][0]["bytes"].is_null());
    assert!(witness["prev"].is_null());
    assert!(
        witness["output_hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("blake3:"))
    );
}

#[test]
fn legacy_home_witness_is_migrated_before_append() {
    let home = unique_path("migration-home");
    let legacy_path = home.join(".epistemic").join("witness.jsonl");
    fs::create_dir_all(legacy_path.parent().expect("legacy parent"))
        .expect("legacy parent should be created");
    fs::write(
        &legacy_path,
        "{\"id\":\"legacy-old\",\"version\":\"witness.v0\",\"tool\":\"hash\",\"outcome\":\"OLD\"}\n",
    )
    .expect("legacy witness should be written");

    let output = support::hashbytes_command("witness-migration")
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env_remove("EPISTEMIC_WITNESS")
        .output()
        .expect("hash binary should run");
    assert!(output.status.success());

    let canonical_path = canonical_witness_path(&home);
    let contents = fs::read_to_string(&canonical_path).expect("canonical witness should exist");
    let lines: Vec<Value> = contents
        .lines()
        .map(|line| serde_json::from_str(line).expect("witness line should parse"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["outcome"], "OLD");
    assert_eq!(lines[1]["outcome"], "ALL_HASHED");
    assert_eq!(lines[1]["prev"], "legacy-old");

    let notice = fs::read_to_string(home.join(".cmdrvl/notices/deprecated-paths.jsonl"))
        .expect("deprecation notice should be written");
    assert!(notice.contains("legacy_path_migrated"));

    let migration = fs::read_to_string(home.join(".cmdrvl/migrations/applied.jsonl"))
        .expect("migration record should be written");
    assert!(migration.contains("copied_legacy_to_canonical"));
}

#[test]
fn file_input_witness_records_manifest_hash_and_size() {
    let temp_dir = unique_path("file-input-dir");
    fs::create_dir_all(&temp_dir).expect("tempdir should be created");
    let witness_path = unique_path("file-input").with_extension("jsonl");
    let data_path = temp_dir.join("data.csv");
    fs::write(&data_path, "loan_id,balance\nL1,100\n").expect("data file should be written");

    let manifest_bytes = format!(
        "{{\"path\":\"{}\",\"version\":\"vacuum.v0\"}}\n",
        data_path.to_string_lossy()
    );
    let manifest_path = temp_dir.join("manifest.jsonl");
    fs::write(&manifest_path, &manifest_bytes).expect("manifest should be written");

    let output = support::hashbytes_command("witness-append-file-input")
        .arg(&manifest_path)
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
    assert_eq!(
        witness["inputs"][0]["path"],
        manifest_path.to_string_lossy().as_ref()
    );
    assert_eq!(witness["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        witness["inputs"][0]["bytes"],
        serde_json::json!(manifest_bytes.len())
    );
    assert_eq!(
        witness["inputs"][0]["hash"],
        format!(
            "blake3:{}",
            blake3::hash(manifest_bytes.as_bytes()).to_hex()
        )
    );

    let _ = fs::remove_file(witness_path);
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn appends_witness_records_across_multiple_runs() {
    let witness_path = unique_path("append-chain").with_extension("jsonl");

    for _ in 0..2 {
        let output = support::hashbytes_command("witness-append-chain")
            .env("EPISTEMIC_WITNESS", &witness_path)
            .output()
            .expect("hash binary should run");
        assert!(output.status.success());
    }

    let contents = fs::read_to_string(&witness_path).expect("witness file should be written");
    let lines: Vec<&str> = contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2);

    let first: Value = serde_json::from_str(lines[0]).expect("first witness line should parse");
    let second: Value = serde_json::from_str(lines[1]).expect("second witness line should parse");
    assert_eq!(first["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(first["tool"], "hash");
    assert!(first["prev"].is_null());
    assert_eq!(second["tool"], "hash");
    assert_eq!(second["prev"], first["id"]);

    let _ = fs::remove_file(witness_path);
}

#[test]
fn no_witness_flag_skips_append() {
    let witness_path = unique_path("disabled").with_extension("jsonl");

    let output = support::hashbytes_command("witness-append-disabled")
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

    let output = support::hashbytes_command("witness-append-failure")
        .env("EPISTEMIC_WITNESS", &witness_dir)
        .output()
        .expect("hash binary should run");

    assert!(output.status.success());

    let _ = fs::remove_dir_all(witness_dir);
}

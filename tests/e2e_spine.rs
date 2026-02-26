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
        "hash-e2e-spine-{}-{suffix}-{nanos}",
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

fn run_hash(manifest_path: &Path, witness_path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--no-witness")
        .arg(manifest_path)
        .env("EPISTEMIC_WITNESS", witness_path)
        .output()
        .expect("hash binary should run")
}

fn parse_jsonl(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8(stdout.to_vec())
        .expect("stdout utf8")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("valid jsonl line"))
        .collect()
}

fn vacuum_like_record(
    path: &Path,
    root: &Path,
    relative_path: &str,
    size: u64,
    extension: &str,
    mime_guess: &str,
) -> Value {
    json!({
        "version": "vacuum.v0",
        "path": path.to_string_lossy().to_string(),
        "relative_path": relative_path,
        "root": root.to_string_lossy().to_string(),
        "size": size,
        "mtime": "2026-01-01T00:00:00.000Z",
        "extension": extension,
        "mime_guess": mime_guess,
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    })
}

fn assert_downstream_compatible(record: &Value) {
    assert_eq!(record["version"], "hash.v0");
    assert!(record.get("path").is_some());
    assert!(record.get("relative_path").is_some());
    assert!(record.get("root").is_some());
    assert!(record.get("bytes_hash").is_some());
    assert!(record.get("hash_algorithm").is_some());
    assert!(record.get("tool_versions").is_some());
}

#[test]
fn hash_output_stays_compatible_with_downstream_fingerprint_and_lock_fields() {
    let root_dir = unique_path("root");
    fs::create_dir_all(&root_dir).expect("create root dir");

    let first_path = root_dir.join("first.csv");
    let second_path = root_dir.join("second.csv");
    fs::write(&first_path, b"loan_id,amount\n1,100\n").expect("write first fixture");
    fs::write(&second_path, b"loan_id,amount\n2,200\n").expect("write second fixture");

    let manifest_path = unique_path("manifest").with_extension("jsonl");
    let witness_path = unique_path("witness").with_extension("jsonl");
    write_jsonl(
        &manifest_path,
        &[
            vacuum_like_record(&first_path, &root_dir, "first.csv", 21, ".csv", "text/csv"),
            vacuum_like_record(
                &second_path,
                &root_dir,
                "second.csv",
                21,
                ".csv",
                "text/csv",
            ),
        ],
    );

    let output = run_hash(&manifest_path, &witness_path);
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["relative_path"], "first.csv");
    assert_eq!(rows[1]["relative_path"], "second.csv");

    for row in rows {
        assert_downstream_compatible(&row);
        assert_eq!(row["hash_algorithm"], "sha256");
        assert!(
            row["bytes_hash"]
                .as_str()
                .expect("bytes_hash string")
                .starts_with("sha256:")
        );
        assert_eq!(row["tool_versions"]["vacuum"], "0.1.0");
        assert!(
            row["tool_versions"]["hash"]
                .as_str()
                .is_some_and(|version| !version.is_empty())
        );
    }

    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
    let _ = fs::remove_file(first_path);
    let _ = fs::remove_file(second_path);
    let _ = fs::remove_dir_all(root_dir);
}

#[test]
fn hash_partial_output_keeps_uniform_schema_for_downstream_tools() {
    let root_dir = unique_path("partial-root");
    fs::create_dir_all(&root_dir).expect("create root dir");

    let existing_path = root_dir.join("present.csv");
    let missing_path = root_dir.join("missing.csv");
    fs::write(&existing_path, b"id,value\n1,a\n").expect("write fixture");

    let manifest_path = unique_path("partial-manifest").with_extension("jsonl");
    let witness_path = unique_path("partial-witness").with_extension("jsonl");
    write_jsonl(
        &manifest_path,
        &[
            json!({
                "version": "vacuum.v0",
                "path": missing_path.to_string_lossy().to_string(),
                "relative_path": "missing.csv",
                "root": root_dir.to_string_lossy().to_string(),
                "size": 0,
                "mtime": "2026-01-01T00:00:00.000Z",
                "extension": ".csv",
                "mime_guess": "text/csv",
                "_skipped": true,
                "_warnings": [
                    {
                        "tool": "vacuum",
                        "code": "E_UPSTREAM",
                        "message": "upstream skipped"
                    }
                ],
                "tool_versions": {
                    "vacuum": "0.1.0"
                }
            }),
            vacuum_like_record(
                &existing_path,
                &root_dir,
                "present.csv",
                13,
                ".csv",
                "text/csv",
            ),
        ],
    );

    let output = run_hash(&manifest_path, &witness_path);
    assert_eq!(output.status.code(), Some(1));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 2);

    assert_downstream_compatible(&rows[0]);
    assert_eq!(rows[0]["_skipped"], true);
    assert_eq!(rows[0]["bytes_hash"], Value::Null);
    assert_eq!(rows[0]["hash_algorithm"], Value::Null);
    assert_eq!(rows[0]["_warnings"][0]["tool"], "vacuum");
    assert_eq!(rows[0]["_warnings"][0]["code"], "E_UPSTREAM");
    assert_eq!(rows[0]["tool_versions"]["vacuum"], "0.1.0");
    assert!(
        rows[0]["tool_versions"]["hash"]
            .as_str()
            .is_some_and(|version| !version.is_empty())
    );

    assert_downstream_compatible(&rows[1]);
    assert_eq!(rows[1]["relative_path"], "present.csv");
    assert_eq!(rows[1]["hash_algorithm"], "sha256");
    assert!(
        rows[1]["bytes_hash"]
            .as_str()
            .expect("bytes_hash string")
            .starts_with("sha256:")
    );

    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
    let _ = fs::remove_file(existing_path);
    let _ = fs::remove_dir_all(root_dir);
}

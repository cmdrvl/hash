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
        "hash-spine-compat-{}-{suffix}-{nanos}",
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

fn run_hash_with_manifest(manifest: &Path, witness: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--no-witness")
        .arg(manifest)
        .env("EPISTEMIC_WITNESS", witness)
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
fn vacuum_records_remain_compatible_for_downstream_fingerprint_lock_fields() {
    let file_a = unique_path("file-a");
    let file_b = unique_path("file-b");
    let manifest_path = unique_path("manifest").with_extension("jsonl");
    let witness_path = unique_path("witness").with_extension("jsonl");

    fs::write(&file_a, b"alpha").expect("write file a");
    fs::write(&file_b, b"beta").expect("write file b");

    let input_rows = vec![
        json!({
            "version": "vacuum.v0",
            "path": file_a.to_string_lossy(),
            "relative_path": "a/alpha.txt",
            "root": "/dataset/root",
            "size": 111,
            "mtime": "2026-01-01T00:00:00.000Z",
            "extension": ".txt",
            "mime_guess": "text/plain",
            "tool_versions": {"vacuum": "0.1.0"},
            "custom_meta": {"source": "fixture-a"}
        }),
        json!({
            "version": "vacuum.v0",
            "path": file_b.to_string_lossy(),
            "relative_path": "b/beta.txt",
            "root": "/dataset/root",
            "size": 222,
            "mtime": "2026-01-02T00:00:00.000Z",
            "extension": ".txt",
            "mime_guess": "text/plain",
            "tool_versions": {"vacuum": "0.1.0"},
            "custom_meta": {"source": "fixture-b"}
        }),
    ];
    write_jsonl(&manifest_path, &input_rows);

    let output = run_hash_with_manifest(&manifest_path, &witness_path);
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 2);

    for (index, row) in rows.iter().enumerate() {
        let input = &input_rows[index];
        assert_eq!(row["version"], "hash.v0");
        assert_eq!(row["path"], input["path"]);
        assert_eq!(row["relative_path"], input["relative_path"]);
        assert_eq!(row["root"], input["root"]);
        assert_eq!(row["size"], input["size"]);
        assert_eq!(row["mtime"], input["mtime"]);
        assert_eq!(row["extension"], input["extension"]);
        assert_eq!(row["mime_guess"], input["mime_guess"]);
        assert_eq!(row["custom_meta"], input["custom_meta"]);
        assert_eq!(row["hash_algorithm"], "sha256");
        assert!(
            row["bytes_hash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("sha256:"))
        );
        assert_eq!(row["tool_versions"]["vacuum"], "0.1.0");
        assert_eq!(row["tool_versions"]["hash"], env!("CARGO_PKG_VERSION"));
    }

    let _ = fs::remove_file(file_a);
    let _ = fs::remove_file(file_b);
    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

#[test]
fn mixed_records_keep_order_and_uniform_hash_fields_for_lock_consumers() {
    let existing = unique_path("existing");
    let missing = unique_path("missing");
    let manifest_path = unique_path("mixed-manifest").with_extension("jsonl");
    let witness_path = unique_path("mixed-witness").with_extension("jsonl");

    fs::write(&existing, b"content").expect("write existing file");
    let _ = fs::remove_file(&missing);

    let input_rows = vec![
        json!({
            "version": "vacuum.v0",
            "path": existing.to_string_lossy(),
            "relative_path": "ok.csv",
            "tool_versions": {"vacuum": "0.1.0"}
        }),
        json!({
            "version": "vacuum.v0",
            "path": "/tmp/upstream-skip.csv",
            "relative_path": "upstream.csv",
            "_skipped": true,
            "_warnings": [{"tool": "vacuum", "code": "E_UPSTREAM"}],
            "tool_versions": {"vacuum": "0.1.0"}
        }),
        json!({
            "version": "vacuum.v0",
            "path": missing.to_string_lossy(),
            "relative_path": "missing.csv",
            "tool_versions": {"vacuum": "0.1.0"}
        }),
    ];
    write_jsonl(&manifest_path, &input_rows);

    let output = run_hash_with_manifest(&manifest_path, &witness_path);
    assert_eq!(output.status.code(), Some(1));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["relative_path"], "ok.csv");
    assert_eq!(rows[1]["relative_path"], "upstream.csv");
    assert_eq!(rows[2]["relative_path"], "missing.csv");

    assert!(rows[0]["bytes_hash"].as_str().is_some());
    assert_eq!(rows[0]["hash_algorithm"], "sha256");
    assert_eq!(rows[0]["_skipped"], Value::Null);

    assert_eq!(rows[1]["_skipped"], true);
    assert_eq!(rows[1]["bytes_hash"], Value::Null);
    assert_eq!(rows[1]["hash_algorithm"], Value::Null);
    assert_eq!(rows[1]["_warnings"][0]["tool"], "vacuum");
    assert_eq!(rows[1]["_warnings"][0]["code"], "E_UPSTREAM");

    assert_eq!(rows[2]["_skipped"], true);
    assert_eq!(rows[2]["bytes_hash"], Value::Null);
    assert_eq!(rows[2]["hash_algorithm"], Value::Null);
    assert_eq!(rows[2]["_warnings"][0]["tool"], "hash");
    assert_eq!(rows[2]["_warnings"][0]["code"], "E_IO");

    for row in rows {
        assert_eq!(row["version"], "hash.v0");
        assert!(row.get("bytes_hash").is_some());
        assert!(row.get("hash_algorithm").is_some());
        assert_eq!(row["tool_versions"]["vacuum"], "0.1.0");
        assert_eq!(row["tool_versions"]["hash"], env!("CARGO_PKG_VERSION"));
    }

    let _ = fs::remove_file(existing);
    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

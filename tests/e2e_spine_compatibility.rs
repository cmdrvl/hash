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
    path.push(format!("hash-e2e-{}-{suffix}-{nanos}", std::process::id()));
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

fn run_hash_on_manifest(manifest_path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--no-witness")
        .arg(manifest_path.to_str().expect("manifest path utf8"))
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

fn mock_vacuum_with_skipped_output() -> Vec<Value> {
    vec![
        json!({
            "version": "vacuum.v0",
            "path": "/data/accessible.txt",
            "relative_path": "accessible.txt",
            "root": "/data",
            "size": 512,
            "mtime": "2025-12-31T12:00:00.000Z",
            "tool_versions": {
                "vacuum": "0.1.0"
            }
        }),
        json!({
            "version": "vacuum.v0",
            "path": "/data/permission_denied.txt",
            "relative_path": "permission_denied.txt",
            "root": "/data",
            "size": 1024,
            "mtime": "2025-12-31T12:00:00.000Z",
            "_skipped": true,
            "_warnings": [
                {
                    "tool": "vacuum",
                    "code": "E_PERMISSION_DENIED",
                    "message": "Permission denied accessing file",
                    "detail": {"path": "/data/permission_denied.txt"}
                }
            ],
            "tool_versions": {
                "vacuum": "0.1.0"
            }
        }),
    ]
}

#[test]
fn hash_preserves_all_vacuum_metadata_fields() {
    let data_path = unique_path("spine-data");
    let manifest_path = unique_path("spine-manifest").with_extension("jsonl");

    fs::write(&data_path, b"spine compatibility test data").expect("write test file");

    let vacuum_record = json!({
        "version": "vacuum.v0",
        "path": data_path.to_string_lossy(),
        "relative_path": "test.txt",
        "root": data_path.parent().unwrap().to_string_lossy(),
        "size": 27,
        "mtime": "2025-12-31T12:00:00.000Z",
        "extension": ".txt",
        "mime_guess": "text/plain",
        "custom_field": "custom_value",
        "tool_versions": {
            "vacuum": "0.1.0"
        }
    });

    write_jsonl(&manifest_path, std::slice::from_ref(&vacuum_record));
    let output = run_hash_on_manifest(&manifest_path);
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    let hash_record = &rows[0];

    // Verify version was updated to hash.v0
    assert_eq!(hash_record["version"], "hash.v0");

    // Verify all vacuum metadata was preserved
    assert_eq!(hash_record["path"], vacuum_record["path"]);
    assert_eq!(hash_record["relative_path"], "test.txt");
    assert_eq!(hash_record["root"], vacuum_record["root"]);
    assert_eq!(hash_record["size"], 27);
    assert_eq!(hash_record["mtime"], "2025-12-31T12:00:00.000Z");
    assert_eq!(hash_record["extension"], ".txt");
    assert_eq!(hash_record["mime_guess"], "text/plain");
    assert_eq!(hash_record["custom_field"], "custom_value");

    // Verify hash fields were added
    assert!(
        hash_record["bytes_hash"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(hash_record["hash_algorithm"], "sha256");

    // Verify tool_versions accumulation
    assert_eq!(hash_record["tool_versions"]["vacuum"], "0.1.0");
    assert_eq!(
        hash_record["tool_versions"]["hash"],
        env!("CARGO_PKG_VERSION")
    );

    let _ = fs::remove_file(data_path);
    let _ = fs::remove_file(manifest_path);
}

#[test]
fn hash_output_provides_fingerprint_required_schema() {
    let data_path = unique_path("fingerprint-schema");
    let manifest_path = unique_path("fingerprint-manifest").with_extension("jsonl");

    fs::write(&data_path, b"fingerprint schema test").expect("write test file");
    write_jsonl(
        &manifest_path,
        &[json!({
            "version": "vacuum.v0",
            "path": data_path.to_string_lossy(),
            "tool_versions": {"vacuum": "0.1.0"}
        })],
    );

    let output = run_hash_on_manifest(&manifest_path);
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    let record = &rows[0];

    // Verify fingerprint expects these fields to be present
    assert!(record.get("version").is_some());
    assert!(record.get("path").is_some());
    assert!(record.get("bytes_hash").is_some());
    assert!(record.get("hash_algorithm").is_some());
    assert!(record.get("tool_versions").is_some());

    // Verify bytes_hash is properly formatted for fingerprint consumption
    let bytes_hash = record["bytes_hash"].as_str().unwrap();
    assert!(bytes_hash.contains(':'));
    let parts: Vec<&str> = bytes_hash.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert!(["sha256", "blake3"].contains(&parts[0]));
    assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));

    let _ = fs::remove_file(data_path);
    let _ = fs::remove_file(manifest_path);
}

#[test]
fn hash_preserves_upstream_skipped_records_for_lock_compatibility() {
    let manifest_path = unique_path("lock-compat").with_extension("jsonl");
    write_jsonl(&manifest_path, &mock_vacuum_with_skipped_output());

    let output = run_hash_on_manifest(&manifest_path);
    assert_eq!(output.status.code(), Some(1)); // PARTIAL due to skipped record

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 2);

    // First record should be hashed (assuming file doesn't exist, it becomes skipped)
    let first_record = &rows[0];
    assert_eq!(first_record["version"], "hash.v0");
    assert_eq!(first_record["_skipped"], true); // File doesn't exist so skipped

    // Second record should preserve upstream skipped status
    let skipped_record = &rows[1];
    assert_eq!(skipped_record["version"], "hash.v0");
    assert_eq!(skipped_record["_skipped"], true);
    assert_eq!(skipped_record["bytes_hash"], Value::Null);
    assert_eq!(skipped_record["hash_algorithm"], Value::Null);

    // Verify upstream warnings are preserved
    let warnings = skipped_record["_warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0]["tool"], "vacuum");
    assert_eq!(warnings[0]["code"], "E_PERMISSION_DENIED");

    // Verify lock expects uniform schema (both records have hash fields)
    for record in &rows {
        assert!(record.get("bytes_hash").is_some());
        assert!(record.get("hash_algorithm").is_some());
        assert!(record.get("_skipped").is_some());
    }

    let _ = fs::remove_file(manifest_path);
}

#[test]
fn hash_output_maintains_deterministic_field_ordering() {
    let data_path = unique_path("deterministic");
    let manifest_path = unique_path("deterministic-manifest").with_extension("jsonl");

    fs::write(&data_path, b"deterministic test").expect("write test file");
    write_jsonl(
        &manifest_path,
        &[json!({
            "version": "vacuum.v0",
            "path": data_path.to_string_lossy(),
            "size": 17,
            "tool_versions": {"vacuum": "0.1.0"}
        })],
    );

    // Run hash twice and verify output is identical
    let output1 = run_hash_on_manifest(&manifest_path);
    let output2 = run_hash_on_manifest(&manifest_path);

    assert_eq!(output1.status.code(), Some(0));
    assert_eq!(output2.status.code(), Some(0));
    assert_eq!(output1.stdout, output2.stdout);

    let _ = fs::remove_file(data_path);
    let _ = fs::remove_file(manifest_path);
}

#[test]
fn hash_tool_versions_accumulate_correctly_across_pipeline() {
    let data_path = unique_path("pipeline-versions");
    let manifest_path = unique_path("pipeline-manifest").with_extension("jsonl");

    fs::write(&data_path, b"tool version accumulation").expect("write test file");

    // Simulate a record that has been through multiple pipeline stages
    let multi_tool_record = json!({
        "version": "shape.v0",
        "path": data_path.to_string_lossy(),
        "tool_versions": {
            "vacuum": "0.1.0",
            "shape": "0.2.0",
            "filter": "0.1.5"
        }
    });

    write_jsonl(&manifest_path, &[multi_tool_record]);
    let output = run_hash_on_manifest(&manifest_path);
    assert_eq!(output.status.code(), Some(0));

    let rows = parse_jsonl(&output.stdout);
    assert_eq!(rows.len(), 1);
    let record = &rows[0];

    // Verify tool_versions accumulation preserves all upstream tools
    let tool_versions = record["tool_versions"].as_object().unwrap();
    assert_eq!(tool_versions["vacuum"], "0.1.0");
    assert_eq!(tool_versions["shape"], "0.2.0");
    assert_eq!(tool_versions["filter"], "0.1.5");
    assert_eq!(tool_versions["hash"], env!("CARGO_PKG_VERSION"));
    assert_eq!(tool_versions.len(), 4);

    let _ = fs::remove_file(data_path);
    let _ = fs::remove_file(manifest_path);
}

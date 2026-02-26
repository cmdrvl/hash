use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_path(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    path.push(format!(
        "hash-progress-output-{}-{suffix}-{nanos}",
        std::process::id()
    ));
    path
}

fn write_jsonl(path: &Path, rows: &[Value]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create manifest parent");
    }

    let mut file = fs::File::create(path).expect("create manifest file");
    for row in rows {
        let line = serde_json::to_string(row).expect("serialize row");
        writeln!(file, "{line}").expect("write row");
    }
}

fn parse_jsonl(bytes: &[u8]) -> Vec<Value> {
    String::from_utf8(bytes.to_vec())
        .expect("utf8 output")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("json line"))
        .collect()
}

#[test]
fn progress_flag_emits_structured_progress_and_warning_events_on_stderr() {
    let existing_path = unique_path("existing");
    let missing_path = unique_path("missing");
    let manifest_path = unique_path("manifest").with_extension("jsonl");
    let witness_path = unique_path("witness").with_extension("jsonl");

    fs::write(&existing_path, b"progress fixture").expect("write fixture");
    let _ = fs::remove_file(&missing_path);

    write_jsonl(
        &manifest_path,
        &[
            json!({
                "version": "vacuum.v0",
                "path": existing_path.to_string_lossy().to_string(),
                "tool_versions": {"vacuum": "0.1.0"}
            }),
            json!({
                "version": "vacuum.v0",
                "path": missing_path.to_string_lossy().to_string(),
                "tool_versions": {"vacuum": "0.1.0"}
            }),
        ],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_hash"))
        .arg("--progress")
        .arg("--no-witness")
        .arg(&manifest_path)
        .env("EPISTEMIC_WITNESS", &witness_path)
        .output()
        .expect("hash binary should run");

    assert_eq!(output.status.code(), Some(1));

    let stdout_rows = parse_jsonl(&output.stdout);
    assert_eq!(stdout_rows.len(), 2);
    assert!(stdout_rows.iter().all(|row| row["version"] == "hash.v0"));

    let stderr_rows = parse_jsonl(&output.stderr);
    let progress_rows: Vec<&Value> = stderr_rows
        .iter()
        .filter(|row| row["type"] == "progress")
        .collect();
    assert!(!progress_rows.is_empty());
    for row in progress_rows {
        assert_eq!(row["tool"], "hash");
        assert!(row["processed"].as_u64().is_some());
        assert!(row["total"].as_u64().is_some());
        assert!(row["percent"].as_f64().is_some());
        assert!(row["elapsed_ms"].as_u64().is_some());
    }

    let warning_rows: Vec<&Value> = stderr_rows
        .iter()
        .filter(|row| row["type"] == "warning")
        .collect();
    assert_eq!(warning_rows.len(), 1);
    assert_eq!(warning_rows[0]["tool"], "hash");
    assert_eq!(
        warning_rows[0]["path"],
        Value::String(missing_path.to_string_lossy().to_string())
    );
    assert!(
        warning_rows[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("skipped:"))
    );

    let _ = fs::remove_file(existing_path);
    let _ = fs::remove_file(manifest_path);
    let _ = fs::remove_file(witness_path);
}

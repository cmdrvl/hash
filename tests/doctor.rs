use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn isolated_command(label: &str) -> (Command, PathBuf) {
    let (command, witness_path, _) = isolated_command_with_root(label);
    (command, witness_path)
}

fn isolated_command_with_root(label: &str) -> (Command, PathBuf, PathBuf) {
    let unique_id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hashbytes-doctor-{label}-{}-{unique_id}",
        std::process::id()
    ));
    let witness_path = root.join("witness").join("witness.jsonl");

    let mut command = Command::new(env!("CARGO_BIN_EXE_hashbytes"));
    command.env("EPISTEMIC_WITNESS", &witness_path);
    command.env("HOME", root.join("home"));
    command.env("USERPROFILE", root.join("profile"));

    (command, witness_path, root)
}

fn parse_stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("doctor stdout must be valid JSON")
}

fn create_executable(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("executable parent should be creatable");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("test executable should be writable");
    file.write_all(b"#!/bin/sh\nexit 0\n")
        .expect("test executable should be writable");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .expect("test executable should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .expect("test executable permissions should be writable");
    }
}

fn write_guard_hooks(home: &Path, dcg_command: &str) {
    let settings_path = home.join(".claude/settings.json");
    let veil_path = home.join("bin/veil");
    create_executable(&veil_path);
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).expect("settings parent should be creatable");
    }
    fs::write(
        settings_path,
        serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Read", "hooks": [{ "type": "command", "command": veil_path.display().to_string() }] },
                    { "matcher": "Grep", "hooks": [{ "type": "command", "command": veil_path.display().to_string() }] },
                    {
                        "matcher": "Bash",
                        "hooks": [
                            { "type": "command", "command": veil_path.display().to_string() },
                            { "type": "command", "command": dcg_command }
                        ]
                    }
                ]
            }
        })
        .to_string(),
    )
    .expect("settings should be writable");
}

fn write_healthy_guard_hooks(home: &Path) {
    let dcg_path = home.join("bin/dcg");
    create_executable(&dcg_path);
    write_guard_hooks(home, &dcg_path.display().to_string());
}

#[test]
fn doctor_health_json_exits_zero_without_writing_witness() {
    let (mut command, witness_path) = isolated_command("health");
    let output = command
        .args(["doctor", "health", "--json"])
        .output()
        .expect("hashbytes doctor health should run");

    assert!(output.status.success());
    let report = parse_stdout_json(&output);
    assert_eq!(report["$schema"], "hashbytes.doctor.health.v1");
    assert_eq!(report["tool"], "hashbytes");
    assert_eq!(report["ok"], true);
    assert_eq!(report["fix_mode"]["available"], false);
    assert!(!witness_path.exists());
    if let Some(parent) = witness_path.parent() {
        assert!(!parent.exists());
    }
}

#[test]
fn doctor_capabilities_json_advertises_no_fixers_or_side_effects() {
    let (mut command, witness_path) = isolated_command("capabilities");
    let output = command
        .args(["doctor", "capabilities", "--json"])
        .output()
        .expect("hashbytes doctor capabilities should run");

    assert!(output.status.success());
    let report = parse_stdout_json(&output);
    assert_eq!(report["$schema"], "hashbytes.doctor.capabilities.v1");
    assert_eq!(report["doctor"]["read_only"], true);
    assert_eq!(report["doctor"]["fix_mode"]["available"], false);
    assert_eq!(
        report["doctor"]["fix_mode"]["fixers"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );

    let side_effects = report["side_effects"]
        .as_object()
        .expect("side_effects must be an object");
    for (name, value) in side_effects {
        assert_eq!(value.as_bool(), Some(false), "{name} must be false");
    }

    assert!(!witness_path.exists());
}

#[test]
fn doctor_robot_triage_json_is_machine_readable() {
    let (mut command, witness_path) = isolated_command("triage");
    let output = command
        .args(["doctor", "--robot-triage"])
        .output()
        .expect("hashbytes doctor robot triage should run");

    assert!(output.status.success());
    let report = parse_stdout_json(&output);
    assert_eq!(report["$schema"], "hashbytes.doctor.triage.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["fix_mode"]["available"], false);
    assert!(
        report["recommended_next_commands"]
            .as_array()
            .is_some_and(|commands| commands.iter().any(|command| command
                .as_str()
                .is_some_and(|command| command == "hashbytes doctor health --json")))
    );
    assert!(!witness_path.exists());
}

#[test]
fn doctor_robot_docs_is_plain_text() {
    let (mut command, witness_path) = isolated_command("robot-docs");
    let output = command
        .args(["doctor", "robot-docs"])
        .output()
        .expect("hashbytes doctor robot-docs should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("hashbytes doctor robot-docs"));
    assert!(stdout.contains("Fix mode: unavailable"));
    assert!(!witness_path.exists());
}

#[test]
fn doctor_fix_is_not_available() {
    let (mut command, witness_path) = isolated_command("fix");
    let output = command
        .args(["doctor", "--fix"])
        .output()
        .expect("hashbytes doctor --fix should run and fail in parser");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unexpected argument '--fix'"));
    assert!(!witness_path.exists());
}

#[test]
fn describe_runs_without_guard_hooks() {
    let (mut command, witness_path) = isolated_command("describe");
    let output = command
        .arg("--describe")
        .output()
        .expect("hashbytes --describe should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("\"name\": \"hashbytes\""));
    assert!(!witness_path.exists());
}

#[test]
fn domain_hashing_fails_closed_without_guard_hooks() {
    let (mut command, witness_path, _) = isolated_command_with_root("guard-missing");
    let output = command
        .arg("--no-witness")
        .output()
        .expect("hashbytes domain command should run");

    assert_eq!(output.status.code(), Some(2));
    let refusal = parse_stdout_json(&output);
    assert_eq!(refusal["refusal"]["code"], "E_GUARD_PREFLIGHT");
    assert!(
        refusal["refusal"]["detail"]["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| finding
                .as_str()
                .is_some_and(|finding| finding.contains("dcg Bash hook is missing"))))
    );
    assert!(!witness_path.exists());
}

#[test]
fn domain_hashing_fails_closed_with_invalid_dcg_hook() {
    let (mut command, witness_path, root) = isolated_command_with_root("guard-invalid-dcg");
    write_guard_hooks(&root.join("home"), "/definitely/missing/dcg");

    let output = command
        .arg("--no-witness")
        .output()
        .expect("hashbytes domain command should run");

    assert_eq!(output.status.code(), Some(2));
    let refusal = parse_stdout_json(&output);
    assert_eq!(refusal["refusal"]["code"], "E_GUARD_PREFLIGHT");
    assert!(
        refusal["refusal"]["detail"]["findings"]
            .as_array()
            .is_some_and(|findings| findings.iter().any(|finding| finding
                .as_str()
                .is_some_and(|finding| finding.contains("dcg Bash hook command"))))
    );
    assert!(!witness_path.exists());
}

#[test]
fn domain_hashing_runs_when_guard_hooks_are_healthy() {
    let (mut command, witness_path, root) = isolated_command_with_root("guard-healthy");
    write_healthy_guard_hooks(&root.join("home"));

    let output = command
        .arg("--no-witness")
        .output()
        .expect("hashbytes domain command should run");

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "empty stdin should produce no hashed records"
    );
    assert!(!witness_path.exists());
}

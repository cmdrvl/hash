use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

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
fn top_level_capabilities_json_advertises_agent_surfaces() {
    let (mut command, witness_path) = isolated_command("top-level-capabilities");
    let output = command
        .args(["capabilities", "--json"])
        .output()
        .expect("hashbytes capabilities should run");

    assert!(output.status.success());
    let report = parse_stdout_json(&output);
    assert_eq!(report["$schema"], "hashbytes.doctor.capabilities.v1");
    assert_eq!(
        report["agent_surfaces"]["robot_triage"]["argv"][0],
        "hashbytes"
    );
    assert_eq!(
        report["agent_surfaces"]["capabilities"]["argv"][1],
        "capabilities"
    );
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
                .is_some_and(|command| command == "hashbytes --robot-triage")))
    );
    assert!(!witness_path.exists());
}

#[test]
fn top_level_robot_triage_is_machine_readable() {
    let (mut command, witness_path) = isolated_command("top-level-triage");
    let output = command
        .arg("--robot-triage")
        .output()
        .expect("hashbytes --robot-triage should run");

    assert!(output.status.success());
    let report = parse_stdout_json(&output);
    assert_eq!(report["$schema"], "hashbytes.doctor.triage.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(
        report["capabilities"]["agent_surfaces"]["robot_docs"]["argv"][2],
        "guide"
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
    assert!(stdout.contains("hashbytes robot-docs guide"));
    assert!(stdout.contains("Fix mode: unavailable"));
    assert!(!witness_path.exists());
}

#[test]
fn top_level_robot_docs_guide_is_plain_text() {
    let (mut command, witness_path) = isolated_command("top-level-robot-docs");
    let output = command
        .args(["robot-docs", "guide"])
        .output()
        .expect("hashbytes robot-docs guide should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("hashbytes robot-docs guide"));
    assert!(stdout.contains("hashbytes capabilities --json"));
    assert!(!witness_path.exists());
}

#[test]
fn doctor_fix_is_not_available() {
    let (mut command, witness_path) = isolated_command("fix");
    let output = command
        .args(["doctor", "--fix"])
        .output()
        .expect("hashbytes doctor --fix should run and report unavailable");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("doctor --fix is unavailable"));
    assert!(stderr.contains("hashbytes --robot-triage"));
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
fn domain_hashing_runs_without_guard_hooks() {
    let (mut command, witness_path, _) = isolated_command_with_root("no-guard-hooks");

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

#[test]
fn top_level_json_is_noop_for_empty_stream() {
    let (mut command, witness_path, _) = isolated_command_with_root("json-noop");

    let output = command
        .args(["--json", "--no-witness"])
        .output()
        .expect("hashbytes --json should run");

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "empty stdin should still produce no hashed records"
    );
    assert!(!witness_path.exists());
}

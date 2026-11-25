use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn isolated_command(label: &str) -> (Command, PathBuf) {
    let unique_id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hashbytes-doctor-{label}-{}-{unique_id}",
        std::process::id()
    ));
    let witness_path = root.join("witness").join("witness.jsonl");
    let mut command = Command::new(env!("CARGO_BIN_EXE_hashbytes"));
    command
        .env("EPISTEMIC_WITNESS", &witness_path)
        .env("HOME", root.join("home"))
        .env("USERPROFILE", root.join("profile"));
    (command, witness_path)
}

fn parse_stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("doctor stdout must be valid JSON")
}

fn assert_doctor_report(args: &[&str], mode: &str) -> (Value, PathBuf, Output) {
    let (mut command, witness_path) = isolated_command(mode);
    let output = command
        .args(args)
        .output()
        .expect("doctor command should run");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = parse_stdout_json(&output);
    assert_eq!(report["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(report["mode"], mode);
    assert_eq!(report["tool"], "hashbytes");
    assert_eq!(report["tool_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(report["healthy"], true);
    assert!(report["findings"].is_array());
    assert!(!witness_path.exists());
    (report, witness_path, output)
}

#[test]
fn doctor_health_uses_family_schema_and_is_read_only() {
    let _ = assert_doctor_report(&["doctor", "--json"], "health");
}

#[test]
fn legacy_doctor_health_alias_remains_supported() {
    let (report, _, _) = assert_doctor_report(&["doctor", "health", "--json"], "health");
    assert_eq!(report["findings"].as_array().map(Vec::len), Some(5));
}

#[test]
fn doctor_capabilities_embeds_the_exact_operator_contract() {
    let (report, _, _) =
        assert_doctor_report(&["doctor", "capabilities", "--json"], "capabilities");
    let (mut command, _) = isolated_command("capabilities-contract");
    let output = command
        .args(["capabilities", "--json"])
        .output()
        .expect("capabilities should run");
    let capabilities = parse_stdout_json(&output);
    assert_eq!(report["contract"], capabilities);
    assert_eq!(capabilities["schema_version"], "operator.v1");
}

#[test]
fn doctor_triage_has_next_actions_and_old_alias_is_supported() {
    let (report, _, _) = assert_doctor_report(&["doctor", "triage", "--json"], "triage");
    assert!(report["next_actions"].as_array().is_some_and(|actions| {
        actions
            .iter()
            .any(|action| action == "hashbytes capabilities --json")
    }));

    let (legacy, _, _) = assert_doctor_report(&["doctor", "--robot-triage"], "triage");
    assert_eq!(legacy["next_actions"], report["next_actions"]);
}

#[test]
fn top_level_surfaces_are_read_only_and_match_the_contract() {
    let (mut describe, witness_path) = isolated_command("describe");
    let described = describe
        .arg("--describe")
        .output()
        .expect("--describe should run");
    assert!(described.status.success());
    assert_eq!(
        parse_stdout_json(&described)["schema_version"],
        "operator.v1"
    );

    let (mut triage, _) = isolated_command("top-level-triage");
    let triage_output = triage
        .arg("--robot-triage")
        .output()
        .expect("--robot-triage should run");
    assert!(triage_output.status.success());
    assert_eq!(parse_stdout_json(&triage_output)["mode"], "triage");
    assert!(!witness_path.exists());
}

#[test]
fn doctor_fix_is_unavailable_and_returns_usage_error() {
    let (mut command, witness_path) = isolated_command("fix");
    let output = command
        .args(["doctor", "--fix"])
        .output()
        .expect("doctor --fix should run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("doctor --fix is unavailable"));
    assert!(!witness_path.exists());
}

#[test]
fn no_color_and_non_tty_help_are_ansi_free() {
    for no_color in [true, false] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hashbytes"));
        command.arg("--help");
        if no_color {
            command.env("NO_COLOR", "1").env("TERM", "dumb");
        } else {
            command.env_remove("NO_COLOR").env_remove("TERM");
        }
        let output = command.output().expect("help should run");
        assert!(!output.stdout.contains(&0x1b), "help stdout contained ANSI");
        assert!(!output.stderr.contains(&0x1b), "help stderr contained ANSI");
    }
}

#[test]
fn typo_recovery_keeps_edit_distance_one_hints_for_flags_and_subcommands() {
    for args in [
        ["--algoritm", "sha256"].as_slice(),
        ["doctor", "healt"].as_slice(),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_hashbytes"))
            .args(args)
            .output()
            .expect("typo invocation should run");
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("similar"), "missing typo hint: {stderr}");
    }
}

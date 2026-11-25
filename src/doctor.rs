use crate::{cli, witness};
use serde_json::{Value, json};
use std::str::FromStr;

const DOCTOR_SCHEMA_VERSION: &str = "cmdrvl.doctor.v1";
const CONTRACT_VERSION: &str = "1.0.0";
const OPERATOR_MANIFEST: &str = include_str!("../operator.json");

pub fn handle_command(
    action: Option<&cli::DoctorAction>,
    robot_triage: bool,
    json_output: bool,
    fix: bool,
) -> u8 {
    if fix {
        return emit_fix_unavailable();
    }

    if robot_triage {
        return emit_robot_triage();
    }

    match action {
        Some(cli::DoctorAction::Health { json }) => emit_health(*json || json_output),
        Some(cli::DoctorAction::Capabilities { json }) => {
            emit_doctor_capabilities(*json || json_output)
        }
        Some(cli::DoctorAction::Triage { json }) => emit_triage(*json || json_output),
        Some(cli::DoctorAction::RobotDocs) => emit_robot_docs(None),
        None => emit_health(json_output),
    }
}

/// Print the canonical operator contract used by both `--describe` and
/// `capabilities --json`.
pub fn emit_capabilities(json_output: bool) -> u8 {
    if json_output {
        print_operator_json();
    } else {
        print_capabilities_text();
    }
    0
}

pub fn emit_operator_json() -> u8 {
    print_operator_json();
    0
}

fn emit_health(json_output: bool) -> u8 {
    let report = doctor_report("health");
    if json_output {
        print_json(&report);
    } else {
        print_health_text(&report);
    }
    doctor_exit_code(&report)
}

fn emit_doctor_capabilities(json_output: bool) -> u8 {
    let report = doctor_report("capabilities");
    if json_output {
        print_json(&report);
    } else {
        println!("hashbytes doctor capabilities");
        println!("contract: hashbytes capabilities --json");
        println!("healthy: {}", report["healthy"]);
    }
    doctor_exit_code(&report)
}

fn emit_triage(json_output: bool) -> u8 {
    let report = doctor_report("triage");
    if json_output {
        print_json(&report);
    } else {
        print_health_text(&report);
        println!("next: hashbytes capabilities --json");
    }
    doctor_exit_code(&report)
}

pub fn emit_robot_triage() -> u8 {
    emit_triage(true)
}

fn doctor_report(mode: &str) -> Value {
    let findings = health_findings();
    let healthy = !findings
        .iter()
        .any(|finding| finding.get("severity").and_then(Value::as_str) == Some("error"));
    let mut report = json!({
        "schema_version": DOCTOR_SCHEMA_VERSION,
        "mode": mode,
        "tool": "hashbytes",
        "tool_version": env!("CARGO_PKG_VERSION"),
        "healthy": healthy,
        "findings": findings,
    });

    match mode {
        "capabilities" => {
            report["contract"] = operator_contract();
        }
        "triage" => {
            let next_actions: &[&str] = if healthy {
                &["hashbytes capabilities --json"]
            } else {
                &[
                    "hashbytes doctor --json",
                    "hashbytes doctor capabilities --json",
                ]
            };
            report["next_actions"] = json!(next_actions);
        }
        "health" => {}
        _ => unreachable!("doctor mode is fixed by the caller"),
    }

    report
}

fn health_findings() -> Vec<Value> {
    [
        operator_manifest_check(),
        schema_contract_check(),
        algorithm_contract_check(),
        stream_contract_check(),
        witness_contract_check(),
    ]
    .into_iter()
    .map(|check| {
        let name = check["name"].as_str().unwrap_or("unknown");
        let ok = check["ok"].as_bool().unwrap_or(false);
        json!({
            "id": format!("hashbytes.{name}"),
            "severity": if ok { "info" } else { "error" },
            "message": if ok {
                format!("{name} check passed")
            } else {
                format!("{name} check failed")
            },
            "fix": if ok { Value::Null } else {
                json!("inspect hashbytes doctor capabilities --json")
            }
        })
    })
    .map(|mut finding| {
        if finding["fix"].is_null() {
            finding
                .as_object_mut()
                .expect("finding object")
                .remove("fix");
        }
        finding
    })
    .collect()
}

fn operator_contract() -> Value {
    serde_json::from_str(OPERATOR_MANIFEST).expect("operator.json must be valid JSON")
}

fn doctor_exit_code(report: &Value) -> u8 {
    if report["healthy"].as_bool() == Some(true) {
        0
    } else {
        1
    }
}

fn operator_manifest_check() -> Value {
    match serde_json::from_str::<Value>(OPERATOR_MANIFEST) {
        Ok(value) => {
            let schema_ok =
                value.get("schema_version").and_then(Value::as_str) == Some("operator.v1");
            let name_ok = value.get("name").and_then(Value::as_str) == Some("hashbytes");
            let version_ok = value
                .get("tool_version")
                .or_else(|| value.get("version"))
                .and_then(Value::as_str)
                == Some(env!("CARGO_PKG_VERSION"));
            let contract_ok =
                value.get("contract_version").and_then(Value::as_str) == Some(CONTRACT_VERSION);
            let commands_ok = ["capabilities", "robot-docs", "doctor"]
                .iter()
                .all(|command| contains_command(&value, command));
            let pipeline_ok = contains_string(&value, &["pipeline", "upstream"], "vacuum")
                && contains_string(&value, &["pipeline", "downstream"], "fingerprint")
                && contains_string(&value, &["pipeline", "downstream"], "lock");
            let ok =
                schema_ok && name_ok && version_ok && contract_ok && commands_ok && pipeline_ok;

            json!({
                "name": "operator_manifest",
                "ok": ok,
                "details": {
                    "schema_version": schema_ok,
                    "name": name_ok,
                    "tool_version": version_ok,
                    "contract_version": contract_ok,
                    "commands": commands_ok,
                    "pipeline": pipeline_ok
                }
            })
        }
        Err(err) => json!({
            "name": "operator_manifest",
            "ok": false,
            "details": { "error": err.to_string() }
        }),
    }
}

fn schema_contract_check() -> Value {
    const SCHEMA: &str = include_str!("../schema/hash.v0.schema.json");
    match serde_json::from_str::<Value>(SCHEMA) {
        Ok(value) => {
            let title_ok = value.get("title").and_then(Value::as_str) == Some("hash.v0");
            let required_ok = [
                "version",
                "path",
                "bytes_hash",
                "hash_algorithm",
                "tool_versions",
            ]
            .iter()
            .all(|field| contains_string(&value, &["required"], field));
            let hash_pattern_ok = value
                .get("properties")
                .and_then(|properties| properties.get("bytes_hash"))
                .and_then(|bytes_hash| bytes_hash.get("pattern"))
                .and_then(Value::as_str)
                == Some("^(sha256|blake3):[a-f0-9]{64}$");
            let ok = title_ok && required_ok && hash_pattern_ok;

            json!({
                "name": "schema_contract",
                "ok": ok,
                "details": {
                    "title": title_ok,
                    "required_fields": required_ok,
                    "bytes_hash_pattern": hash_pattern_ok
                }
            })
        }
        Err(err) => json!({
            "name": "schema_contract",
            "ok": false,
            "details": { "error": err.to_string() }
        }),
    }
}

fn algorithm_contract_check() -> Value {
    let sha256_ok = cli::Algorithm::from_str("sha256").is_ok();
    let blake3_ok = cli::Algorithm::from_str("BLAKE3").is_ok();
    let rejects_unknown = cli::Algorithm::from_str("md5").is_err();
    let ok = sha256_ok && blake3_ok && rejects_unknown;

    json!({
        "name": "algorithm_contract",
        "ok": ok,
        "details": {
            "sha256": sha256_ok,
            "blake3_case_insensitive": blake3_ok,
            "rejects_unknown": rejects_unknown
        }
    })
}

fn stream_contract_check() -> Value {
    json!({
        "name": "stream_contract",
        "ok": true,
        "details": {
            "normal_stdout": "hash.v0 JSONL records or one refusal envelope",
            "doctor_stdout": "doctor report only",
            "reads_stdin": false,
            "reads_input_manifest": false,
            "reads_artifact_bytes": false
        }
    })
}

fn witness_contract_check() -> Value {
    let witness_path = witness::default_witness_path();
    let path_is_usable = !witness_path.is_dir();
    json!({
        "name": "witness_contract",
        "ok": path_is_usable,
        "details": {
            "resolved_path": witness_path.to_string_lossy(),
            "resolved_only": true,
            "opens_ledger": false,
            "appends_ledger": false,
            "creates_directory": false,
            "path_is_usable": path_is_usable
        }
    })
}

fn contains_command(value: &Value, expected: &str) -> bool {
    value
        .get("commands")
        .and_then(Value::as_array)
        .is_some_and(|commands| {
            commands
                .iter()
                .any(|command| command.get("name").and_then(Value::as_str) == Some(expected))
        })
}

fn contains_string(value: &Value, path: &[&str], expected: &str) -> bool {
    let mut current = value;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return false;
        };
        current = next;
    }

    current
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn print_operator_json() {
    print!("{OPERATOR_MANIFEST}");
    if !OPERATOR_MANIFEST.ends_with('\n') {
        println!();
    }
}

fn print_health_text(report: &Value) {
    let healthy = report["healthy"].as_bool().unwrap_or(false);
    println!(
        "hashbytes doctor: {}",
        if healthy { "healthy" } else { "unhealthy" }
    );
    if let Some(findings) = report.get("findings").and_then(Value::as_array) {
        for finding in findings {
            println!(
                "- {}: {}",
                finding["id"].as_str().unwrap_or("unknown"),
                finding["message"].as_str().unwrap_or("unknown")
            );
        }
    }
}

fn print_capabilities_text() {
    println!("hashbytes capabilities");
    println!("operator contract: hashbytes --describe");
    println!("doctor: hashbytes doctor --json");
    println!("triage: hashbytes doctor triage --json");
    println!("robot docs: hashbytes robot-docs guide");
}

pub fn emit_robot_docs(_action: Option<&cli::RobotDocsAction>) -> u8 {
    print_robot_docs();
    0
}

pub fn emit_fix_unavailable() -> u8 {
    eprintln!("hashbytes doctor --fix is unavailable; doctor is read-only in this release.");
    eprintln!("Safe alternatives:");
    eprintln!("- hashbytes doctor triage --json");
    eprintln!("- hashbytes doctor capabilities --json");
    eprintln!("- hashbytes capabilities --json");
    2
}

fn print_robot_docs() {
    println!("hashbytes robot-docs guide");
    println!(
        "Purpose: inspect the compiled hashbytes contract without reading manifests, hashing files, or appending witness records."
    );
    println!("Top-level triage: hashbytes --robot-triage");
    println!("Top-level capabilities: hashbytes capabilities --json");
    println!("Structured hash output: hashbytes --json # accepted; normal stdout is already JSONL");
    println!(
        "Composition: vacuum --json <ROOT>... | hashbytes | fingerprint --fp <ID> | lock --dataset-id <DATASET> > dataset.lock.json"
    );
    println!("Seal: pack seal dataset.lock.json --output evidence/<DATASET>/");
    println!("Health: hashbytes doctor --json");
    println!("Capabilities: hashbytes doctor capabilities --json");
    println!("Triage: hashbytes doctor triage --json");
    println!("Legacy aliases: hashbytes doctor health --json and hashbytes doctor --robot-triage");
    println!("Fix mode: unavailable in this release.");
    println!(
        "Normal hashing still appends witness records unless --no-witness is used; doctor commands do not append witness records."
    );
}

fn print_json(value: &Value) {
    match serde_json::to_string_pretty(value) {
        Ok(rendered) => println!("{rendered}"),
        Err(err) => {
            eprintln!("hashbytes doctor: failed to render JSON: {err}");
            println!(
                "{{\"healthy\":false,\"findings\":[{{\"id\":\"hashbytes.render\",\"severity\":\"error\",\"message\":\"failed to render report\"}}]}}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::doctor_exit_code;
    use serde_json::json;

    #[test]
    fn unhealthy_doctor_reports_are_negative_not_refusals() {
        assert_eq!(doctor_exit_code(&json!({"healthy": false})), 1);
        assert_ne!(doctor_exit_code(&json!({"healthy": false})), 2);
    }
}

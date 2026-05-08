use crate::{cli, witness};
use serde_json::{Value, json};
use std::str::FromStr;

const HEALTH_SCHEMA: &str = "hashbytes.doctor.health.v1";
const CAPABILITIES_SCHEMA: &str = "hashbytes.doctor.capabilities.v1";
const TRIAGE_SCHEMA: &str = "hashbytes.doctor.triage.v1";
const READ_ONLY_CONTRACT: &str = "cmdrvl.read_only_doctor.v1";

pub fn handle_command(
    action: Option<&cli::DoctorAction>,
    robot_triage: bool,
    json_output: bool,
) -> u8 {
    if robot_triage {
        return emit_robot_triage();
    }

    match action {
        Some(cli::DoctorAction::Health { json }) => emit_health(*json || json_output),
        Some(cli::DoctorAction::Capabilities { json }) => emit_capabilities(*json || json_output),
        Some(cli::DoctorAction::RobotDocs) => {
            print_robot_docs();
            0
        }
        None => emit_health(json_output),
    }
}

fn emit_health(json_output: bool) -> u8 {
    let report = health_report();
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);

    if json_output {
        print_json(&report);
    } else {
        print_health_text(&report);
    }

    if ok { 0 } else { 2 }
}

fn emit_capabilities(json_output: bool) -> u8 {
    let report = capabilities_report();

    if json_output {
        print_json(&report);
    } else {
        print_capabilities_text();
    }

    0
}

fn emit_robot_triage() -> u8 {
    let health = health_report();
    let ok = health.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let report = json!({
        "$schema": TRIAGE_SCHEMA,
        "tool": "hashbytes",
        "version": env!("CARGO_PKG_VERSION"),
        "ok": ok,
        "status": if ok { "healthy" } else { "unhealthy" },
        "read_only_contract": READ_ONLY_CONTRACT,
        "fix_mode": {
            "available": false,
            "reason": "hashbytes doctor is audit-only until detectors, backups, inverses, and fixtures exist"
        },
        "health": health,
        "capabilities": capabilities_report(),
        "failure_modes": [
            {
                "id": "hash-doctor-001",
                "name": "doctor-positional-input-ambiguity",
                "detector": "hashbytes doctor health --json",
                "status": if ok { "covered" } else { "failing" }
            },
            {
                "id": "hash-doctor-002",
                "name": "witness-append-regression",
                "detector": "run doctor commands with EPISTEMIC_WITNESS pointed at a nonexistent path",
                "status": "covered_by_tests"
            },
            {
                "id": "hash-doctor-003",
                "name": "accidental-fix-mode",
                "detector": "hashbytes doctor --fix exits with CLI parse error",
                "status": "covered_by_tests"
            }
        ],
        "recommended_next_commands": [
            "hashbytes doctor health --json",
            "hashbytes doctor capabilities --json",
            "hashbytes --describe",
            "hashbytes --schema"
        ]
    });

    print_json(&report);
    if ok { 0 } else { 2 }
}

fn health_report() -> Value {
    let checks = vec![
        operator_manifest_check(),
        schema_contract_check(),
        algorithm_contract_check(),
        stream_contract_check(),
        witness_contract_check(),
    ];
    let ok = checks
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true));

    json!({
        "$schema": HEALTH_SCHEMA,
        "tool": "hashbytes",
        "version": env!("CARGO_PKG_VERSION"),
        "ok": ok,
        "status": if ok { "healthy" } else { "unhealthy" },
        "read_only_contract": READ_ONLY_CONTRACT,
        "fix_mode": {
            "available": false
        },
        "checks": checks
    })
}

fn capabilities_report() -> Value {
    json!({
        "$schema": CAPABILITIES_SCHEMA,
        "tool": "hashbytes",
        "version": env!("CARGO_PKG_VERSION"),
        "read_only_contract": READ_ONLY_CONTRACT,
        "doctor": {
            "read_only": true,
            "fix_mode": {
                "available": false,
                "fixers": []
            },
            "commands": [
                {
                    "name": "health",
                    "argv": ["hashbytes", "doctor", "health"],
                    "json_argv": ["hashbytes", "doctor", "health", "--json"],
                    "description": "Validate the compiled operator, schema, algorithm, stream, and witness contracts"
                },
                {
                    "name": "capabilities",
                    "argv": ["hashbytes", "doctor", "capabilities", "--json"],
                    "description": "Return machine-readable doctor capabilities and side-effect promises"
                },
                {
                    "name": "robot-docs",
                    "argv": ["hashbytes", "doctor", "robot-docs"],
                    "description": "Print concise agent-facing usage notes"
                },
                {
                    "name": "robot-triage",
                    "argv": ["hashbytes", "doctor", "--robot-triage"],
                    "description": "Return one machine-readable health and triage report"
                }
            ]
        },
        "hashbytes_capabilities": {
            "streaming_jsonl": true,
            "algorithms": ["sha256", "blake3"],
            "operator_describe": true,
            "schema_describe": true,
            "witness_query": true,
            "default_witness_append_for_hashing": true
        },
        "side_effects": {
            "reads_stdin": false,
            "reads_input_manifest": false,
            "reads_artifact_bytes": false,
            "hashes_files": false,
            "emits_hash_jsonl": false,
            "opens_witness_ledger": false,
            "appends_witness_ledger": false,
            "creates_witness_directory": false,
            "writes_doctor_artifacts": false,
            "rewrites_operator_manifest": false,
            "rewrites_schema": false,
            "changes_cwd": false,
            "uses_network": false
        }
    })
}

fn operator_manifest_check() -> Value {
    const OPERATOR_MANIFEST: &str = include_str!("../operator.json");
    let parsed = serde_json::from_str::<Value>(OPERATOR_MANIFEST);

    match parsed {
        Ok(value) => {
            let name_ok = value.get("name").and_then(Value::as_str) == Some("hashbytes");
            let version_ok =
                value.get("version").and_then(Value::as_str) == Some(env!("CARGO_PKG_VERSION"));
            let output_mode_ok = value
                .get("invocation")
                .and_then(|invocation| invocation.get("output_mode"))
                .and_then(Value::as_str)
                == Some("stream");
            let output_schema_ok = value
                .get("invocation")
                .and_then(|invocation| invocation.get("output_schema"))
                .and_then(Value::as_str)
                == Some("hash.v0");
            let pipeline_ok = contains_string(&value, &["pipeline", "upstream"], "vacuum")
                && contains_string(&value, &["pipeline", "downstream"], "fingerprint")
                && contains_string(&value, &["pipeline", "downstream"], "lock");
            let ok = name_ok && version_ok && output_mode_ok && output_schema_ok && pipeline_ok;

            json!({
                "name": "operator_manifest",
                "ok": ok,
                "details": {
                    "name": name_ok,
                    "version": version_ok,
                    "output_mode": output_mode_ok,
                    "output_schema": output_schema_ok,
                    "pipeline": pipeline_ok
                }
            })
        }
        Err(err) => json!({
            "name": "operator_manifest",
            "ok": false,
            "details": {
                "error": err.to_string()
            }
        }),
    }
}

fn schema_contract_check() -> Value {
    const SCHEMA: &str = include_str!("../schema/hash.v0.schema.json");
    let parsed = serde_json::from_str::<Value>(SCHEMA);

    match parsed {
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
            "details": {
                "error": err.to_string()
            }
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

    json!({
        "name": "witness_contract",
        "ok": true,
        "details": {
            "resolved_path": witness_path.to_string_lossy(),
            "resolved_only": true,
            "opens_ledger": false,
            "appends_ledger": false,
            "creates_directory": false
        }
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
        .map(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        .unwrap_or(false)
}

fn print_health_text(report: &Value) {
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);
    println!(
        "hashbytes doctor: {}",
        if ok { "healthy" } else { "unhealthy" }
    );
    println!("read_only: true");
    println!("fix_mode: unavailable");

    if let Some(checks) = report.get("checks").and_then(Value::as_array) {
        for check in checks {
            let name = check
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let status = if check.get("ok").and_then(Value::as_bool) == Some(true) {
                "ok"
            } else {
                "failed"
            };
            println!("- {name}: {status}");
        }
    }
}

fn print_capabilities_text() {
    println!("hashbytes doctor capabilities");
    println!("read_only: true");
    println!("fix_mode: unavailable");
    println!("commands:");
    println!("- hashbytes doctor health --json");
    println!("- hashbytes doctor capabilities --json");
    println!("- hashbytes doctor robot-docs");
    println!("- hashbytes doctor --robot-triage");
}

fn print_robot_docs() {
    println!("hashbytes doctor robot-docs");
    println!(
        "Purpose: inspect the compiled hashbytes contract without reading manifests, hashing files, or appending witness records."
    );
    println!("Health: hashbytes doctor health --json");
    println!("Capabilities: hashbytes doctor capabilities --json");
    println!("Triage: hashbytes doctor --robot-triage");
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
            println!("{{\"ok\":false}}");
        }
    }
}

#![forbid(unsafe_code)]

use clap::Parser;

pub mod cli;
pub mod hash;
pub mod output;
pub mod pipeline;
pub mod progress;
pub mod refusal;
pub mod witness;

/// Main entry point that handles all errors internally and returns exit code
pub fn run() -> u8 {
    let cli = cli::Cli::parse();

    // Handle immediate flags that don't require input processing
    if cli.describe {
        print_operator_json();
        return 0;
    }

    if cli.schema {
        print_json_schema();
        return 0;
    }

    // Handle witness subcommands
    if let Some(cli::Command::Witness { action }) = cli.command {
        return handle_witness_command(&action);
    }

    // Handle main hashing workflow
    handle_main_workflow(&cli)
}

fn print_operator_json() {
    let operator = serde_json::json!({
        "schema_version": "operator.v0",
        "name": "hash",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Computes exact byte identity (SHA-256 or BLAKE3) for artifacts in a manifest",
        "repository": "https://github.com/cmdrvl/hash",
        "license": "MIT",
        "invocation": {
            "binary": "hash",
            "output_mode": "stream",
            "output_schema": "hash.v0",
            "json_flag": null
        },
        "arguments": [
            {
                "name": "input",
                "type": "file_path",
                "required": false,
                "position": 0,
                "description": "JSONL manifest file (default: stdin)"
            }
        ],
        "options": [
            {
                "name": "algorithm",
                "flag": "--algorithm",
                "type": "string",
                "default": "sha256",
                "description": "Hash algorithm: sha256 or blake3"
            },
            {
                "name": "jobs",
                "flag": "--jobs",
                "type": "integer",
                "description": "Number of parallel workers (default: CPU count)"
            }
        ],
        "exit_codes": {
            "0": { "meaning": "ALL_HASHED", "domain": "positive" },
            "1": { "meaning": "PARTIAL", "domain": "negative" },
            "2": { "meaning": "REFUSAL", "domain": "error" }
        },
        "refusals": [
            {
                "code": "E_BAD_INPUT",
                "message": "Input is not valid JSONL or missing required fields",
                "action": "escalate"
            },
            {
                "code": "E_IO",
                "message": "Cannot read input/output stream",
                "action": "escalate"
            }
        ],
        "capabilities": {
            "formats": ["*"],
            "profile_aware": false,
            "streaming": true
        },
        "pipeline": {
            "upstream": ["vacuum"],
            "downstream": ["fingerprint", "lock"]
        }
    });

    println!("{}", serde_json::to_string_pretty(&operator).unwrap());
}

fn print_json_schema() {
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "hash.v0",
        "type": "object",
        "properties": {
            "version": {
                "type": "string",
                "const": "hash.v0"
            },
            "path": {
                "type": "string",
                "description": "Absolute file path"
            },
            "bytes_hash": {
                "type": ["string", "null"],
                "pattern": "^(sha256|blake3):[a-f0-9]{64}$",
                "description": "Cryptographic hash with algorithm prefix"
            },
            "hash_algorithm": {
                "type": ["string", "null"],
                "enum": ["sha256", "blake3"]
            },
            "tool_versions": {
                "type": "object",
                "additionalProperties": {
                    "type": "string"
                }
            },
            "_skipped": {
                "type": "boolean",
                "description": "True if file could not be hashed"
            },
            "_warnings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "tool": { "type": "string" },
                        "code": { "type": "string" },
                        "message": { "type": "string" },
                        "detail": { "type": "object" }
                    }
                }
            }
        },
        "required": ["version", "path", "bytes_hash", "hash_algorithm", "tool_versions"]
    });

    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}

fn handle_witness_command(action: &cli::WitnessAction) -> u8 {
    let _ = action;
    cli::exit_code(cli::Outcome::AllHashed)
}

fn handle_main_workflow(cli: &cli::Cli) -> u8 {
    // Validate and parse algorithm
    let _algorithm = match cli.algorithm.parse::<cli::Algorithm>() {
        Ok(alg) => alg,
        Err(err) => {
            let refusal = refusal::RefusalEnvelope::new(
                refusal::RefusalCode::BadInput,
                format!("Invalid algorithm: {}", err),
                serde_json::json!({ "algorithm": cli.algorithm }),
            );
            println!("{}", serde_json::to_string(&refusal).unwrap());
            return 2;
        }
    };

    // TODO: Implement main hashing workflow
    // For now, return success to complete CLI contract implementation
    cli::exit_code(cli::Outcome::AllHashed)
}

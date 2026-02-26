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
    const OPERATOR_MANIFEST: &str = include_str!("../operator.json");
    print!("{OPERATOR_MANIFEST}");

    if !OPERATOR_MANIFEST.ends_with('\n') {
        println!();
    }
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
            let refusal = refusal::RefusalEnvelope::from_code(
                refusal::RefusalCode::BadInput,
                serde_json::json!({
                    "algorithm": cli.algorithm,
                    "error": err
                }),
            );
            println!("{}", serde_json::to_string(&refusal).unwrap());
            return 2;
        }
    };

    // TODO: Implement main hashing workflow
    // For now, return success to complete CLI contract implementation
    cli::exit_code(cli::Outcome::AllHashed)
}

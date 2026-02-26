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
    run_with_cli(cli::Cli::parse())
}

pub fn run_with_cli(cli: cli::Cli) -> u8 {
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
    if let Some(cli::Command::Witness { action }) = &cli.command {
        return handle_witness_command(action);
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
    const SCHEMA_MANIFEST: &str = include_str!("../schema/hash.v0.schema.json");
    print!("{SCHEMA_MANIFEST}");

    if !SCHEMA_MANIFEST.ends_with('\n') {
        println!();
    }
}

fn handle_witness_command(action: &cli::WitnessAction) -> u8 {
    match action {
        cli::WitnessAction::Query { json, .. } => {
            if *json {
                println!("[]");
            }
            cli::exit_code(cli::Outcome::Partial)
        }
        cli::WitnessAction::Last { json } => {
            if *json {
                println!("null");
            }
            cli::exit_code(cli::Outcome::Partial)
        }
        cli::WitnessAction::Count { .. } => {
            println!("0");
            cli::exit_code(cli::Outcome::Partial)
        }
    }
}

fn handle_main_workflow(cli: &cli::Cli) -> u8 {
    // Validate and parse algorithm
    let algorithm = match cli.algorithm.parse::<cli::Algorithm>() {
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

    // Open input source (file or stdin)
    let input_reader: Box<dyn std::io::BufRead> = match &cli.input {
        Some(path) => match std::fs::File::open(path) {
            Ok(file) => Box::new(std::io::BufReader::new(file)),
            Err(err) => {
                let refusal = refusal::RefusalEnvelope::io_error(err.to_string());
                println!("{}", serde_json::to_string(&refusal).unwrap());
                return 2;
            }
        },
        None => Box::new(std::io::BufReader::new(std::io::stdin())),
    };

    // Process JSONL stream
    match process_jsonl_stream(input_reader, algorithm) {
        Ok(outcome) => cli::exit_code(outcome),
        Err(refusal_envelope) => {
            println!("{}", serde_json::to_string(&refusal_envelope).unwrap());
            2
        }
    }
}

fn process_jsonl_stream(
    mut reader: Box<dyn std::io::BufRead>,
    algorithm: cli::Algorithm,
) -> Result<cli::Outcome, Box<refusal::RefusalEnvelope>> {
    use std::io::BufRead;

    let mut line_number = 0;
    let mut buffer = String::new();
    let mut any_skipped = false;

    loop {
        buffer.clear();
        let bytes_read = reader
            .read_line(&mut buffer)
            .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;

        // End of input
        if bytes_read == 0 {
            break;
        }

        line_number += 1;

        // Skip empty lines
        if buffer.trim().is_empty() {
            continue;
        }

        // Parse JSONL line
        let parsed_line = pipeline::reader::parse_json_line(&buffer, line_number)?;
        let mut record = parsed_line.record;

        // Process record: check if already skipped or needs hashing
        if let Some(record_obj) = record.as_object() {
            if pipeline::enricher::is_skipped(record_obj) {
                // Pass through upstream skipped records
                record = pipeline::enricher::process_skipped_record(record);
                any_skipped = true;
            } else {
                // Extract path for hashing
                let path_str = record
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        Box::new(refusal::RefusalEnvelope::bad_input_missing_field(
                            line_number,
                            "path",
                        ))
                    })?
                    .to_owned();

                let path = std::path::Path::new(&path_str);

                // Attempt to hash the file
                match hash::hash_file(path, algorithm) {
                    Ok(file_hash) => {
                        // Successfully hashed - process as normal record
                        record = pipeline::enricher::process_hashed_record(
                            record,
                            file_hash,
                            algorithm.prefix(),
                        );
                    }
                    Err(io_err) => {
                        // IO failure - mark as skipped with warning
                        record = pipeline::enricher::process_io_failed_record(
                            record,
                            &path_str,
                            &io_err.to_string(),
                        );
                        any_skipped = true;
                    }
                }
            }
        }

        // Emit processed record as JSONL
        output::jsonl::write_json_line(&mut std::io::stdout(), &record)
            .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;
    }

    // Determine final outcome based on whether any records were skipped
    if any_skipped {
        Ok(cli::Outcome::Partial)
    } else {
        Ok(cli::Outcome::AllHashed)
    }
}

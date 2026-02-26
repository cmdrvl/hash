#![forbid(unsafe_code)]

use clap::Parser;
use serde_json::{Map, Value};

pub mod cli;
pub mod hash;
pub mod output;
pub mod pipeline;
pub mod progress;
pub mod refusal;
pub mod witness;

struct RunResult {
    outcome: cli::Outcome,
    output_hash: String,
}

impl RunResult {
    fn new(outcome: cli::Outcome, output_hash: String) -> Self {
        Self {
            outcome,
            output_hash,
        }
    }

    fn exit_code(&self) -> u8 {
        self.outcome.exit_code()
    }
}

struct StreamOutcome {
    outcome: cli::Outcome,
    output_hash: String,
}

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
    let result = handle_main_workflow(&cli);
    append_witness_non_fatal(&cli, &result);
    result.exit_code()
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

fn handle_main_workflow(cli: &cli::Cli) -> RunResult {
    // Validate and parse algorithm
    let algorithm = match cli.algorithm.parse::<cli::Algorithm>() {
        Ok(alg) => alg,
        Err(err) => {
            return refusal_result(refusal::RefusalEnvelope::from_code(
                refusal::RefusalCode::BadInput,
                serde_json::json!({
                    "algorithm": cli.algorithm,
                    "error": err
                }),
            ));
        }
    };

    // Open input source (file or stdin)
    let input_reader: Box<dyn std::io::BufRead> = match &cli.input {
        Some(path) => match std::fs::File::open(path) {
            Ok(file) => Box::new(std::io::BufReader::new(file)),
            Err(err) => {
                return refusal_result(refusal::RefusalEnvelope::io_error(err.to_string()));
            }
        },
        None => Box::new(std::io::BufReader::new(std::io::stdin())),
    };

    // Process JSONL stream
    match process_jsonl_stream(input_reader, algorithm) {
        Ok(stream_outcome) => RunResult::new(stream_outcome.outcome, stream_outcome.output_hash),
        Err(refusal_envelope) => refusal_result(*refusal_envelope),
    }
}

fn process_jsonl_stream(
    mut reader: Box<dyn std::io::BufRead>,
    algorithm: cli::Algorithm,
) -> Result<StreamOutcome, Box<refusal::RefusalEnvelope>> {
    use std::io::BufRead;
    use std::io::Write;

    let mut line_number = 0;
    let mut buffer = String::new();
    let mut any_skipped = false;
    let mut stdout = std::io::stdout();
    let mut output_hasher = blake3::Hasher::new();

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
        let mut rendered = Vec::new();
        output::jsonl::write_json_line(&mut rendered, &record)
            .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;
        stdout
            .write_all(&rendered)
            .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;
        output_hasher.update(&rendered);
    }

    // Determine final outcome based on whether any records were skipped
    let outcome = if any_skipped {
        cli::Outcome::Partial
    } else {
        cli::Outcome::AllHashed
    };

    Ok(StreamOutcome {
        outcome,
        output_hash: format!("blake3:{}", output_hasher.finalize().to_hex()),
    })
}

fn refusal_result(refusal: refusal::RefusalEnvelope) -> RunResult {
    let rendered = serde_json::to_string(&refusal).unwrap();
    println!("{rendered}");
    RunResult::new(
        cli::Outcome::Refusal,
        hash_bytes(format!("{rendered}\n").as_bytes()),
    )
}

fn append_witness_non_fatal(cli: &cli::Cli, result: &RunResult) {
    if cli.no_witness {
        return;
    }

    let record = witness::WitnessRecord::from_run(
        outcome_label(result.outcome),
        result.exit_code(),
        witness_params(cli),
        result.output_hash.clone(),
    );

    if let Err(err) = witness::append_default_record(&record) {
        eprintln!("hash: warning: witness append failed: {err}");
    }
}

fn witness_params(cli: &cli::Cli) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("algorithm".to_owned(), Value::String(cli.algorithm.clone()));
    if let Some(jobs) = cli.jobs {
        params.insert("jobs".to_owned(), Value::from(jobs));
    }
    params
}

fn outcome_label(outcome: cli::Outcome) -> &'static str {
    match outcome {
        cli::Outcome::AllHashed => "ALL_HASHED",
        cli::Outcome::Partial => "PARTIAL",
        cli::Outcome::Refusal => "REFUSAL",
    }
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

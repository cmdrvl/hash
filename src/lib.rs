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

struct PendingRecord {
    line_number: usize,
    record: Value,
}

struct ProcessedRecord {
    record: Value,
    warning_event: Option<progress::WarningEvent>,
    skipped: bool,
}

struct StreamState<'a> {
    progress_enabled: bool,
    stdout: &'a mut std::io::Stdout,
    stderr: &'a mut std::io::Stderr,
    output_hasher: &'a mut blake3::Hasher,
    processed: &'a mut usize,
    any_skipped: &'a mut bool,
    progress_started_at: std::time::Instant,
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
    match witness::query::handle_witness_query(action) {
        Ok(exit_code) => exit_code,
        Err(_) => cli::exit_code(cli::Outcome::Refusal),
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

    let jobs = pipeline::parallel::normalized_jobs(cli.jobs);

    // Process JSONL stream
    match process_jsonl_stream(input_reader, algorithm, jobs, cli.progress) {
        Ok(stream_outcome) => RunResult::new(stream_outcome.outcome, stream_outcome.output_hash),
        Err(refusal_envelope) => refusal_result(*refusal_envelope),
    }
}

fn process_jsonl_stream(
    mut reader: Box<dyn std::io::BufRead>,
    algorithm: cli::Algorithm,
    jobs: usize,
    progress_enabled: bool,
) -> Result<StreamOutcome, Box<refusal::RefusalEnvelope>> {
    use std::io::BufRead;

    let mut line_number = 0;
    let mut buffer = String::new();
    let mut any_skipped = false;
    let mut processed = 0usize;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let mut output_hasher = blake3::Hasher::new();
    let batch_size = stream_batch_size(jobs);
    let mut pending_records = Vec::with_capacity(batch_size);
    let mut stream_state = StreamState {
        progress_enabled,
        stdout: &mut stdout,
        stderr: &mut stderr,
        output_hasher: &mut output_hasher,
        processed: &mut processed,
        any_skipped: &mut any_skipped,
        progress_started_at: std::time::Instant::now(),
    };

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
        pending_records.push(PendingRecord {
            line_number: parsed_line.line_number,
            record: parsed_line.record,
        });

        if pending_records.len() >= batch_size {
            flush_pending_records(
                std::mem::take(&mut pending_records),
                algorithm,
                jobs,
                &mut stream_state,
            )?;
        }
    }

    flush_pending_records(pending_records, algorithm, jobs, &mut stream_state)?;

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

    let witness_path = witness::default_witness_path();
    let inputs = match witness_inputs(cli) {
        Ok(inputs) => inputs,
        Err(err) => {
            let input_label = cli
                .input
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "stdin".to_owned());
            emit_witness_warning(
                cli,
                &input_label,
                &format!("witness input metadata failed: {err}"),
            );
            return;
        }
    };
    let record = witness::WitnessRecord::from_run(
        inputs,
        outcome_label(result.outcome),
        result.exit_code(),
        witness_params(cli),
        result.output_hash.clone(),
    );

    if let Err(err) = witness::append_record(&witness_path, &record) {
        emit_witness_warning(
            cli,
            &witness_path.to_string_lossy(),
            &format!("witness append failed: {err}"),
        );
    }
}

fn witness_params(cli: &cli::Cli) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("algorithm".to_owned(), Value::String(cli.algorithm.clone()));
    if let Some(jobs) = cli.jobs {
        params.insert(
            "jobs".to_owned(),
            Value::from(pipeline::parallel::normalized_jobs(Some(jobs))),
        );
    }
    params
}

fn witness_inputs(cli: &cli::Cli) -> Result<Vec<witness::record::WitnessInput>, std::io::Error> {
    match &cli.input {
        Some(path) => {
            let bytes = std::fs::read(path)?;
            Ok(vec![witness::WitnessRecord::input(
                path.to_string_lossy().into_owned(),
                Some(hash_bytes(&bytes)),
                Some(bytes.len() as u64),
            )])
        }
        None => Ok(vec![witness::WitnessRecord::input("stdin", None, None)]),
    }
}

fn emit_witness_warning(cli: &cli::Cli, path: &str, message: &str) {
    if cli.progress {
        let warning_event = progress::WarningEvent::new(path, message);
        let _ = progress::write_warning(&mut std::io::stderr(), &warning_event);
    } else {
        eprintln!("hash: warning: {message}");
    }
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

fn stream_batch_size(jobs: usize) -> usize {
    jobs.max(1).saturating_mul(32)
}

fn flush_pending_records(
    pending_records: Vec<PendingRecord>,
    algorithm: cli::Algorithm,
    jobs: usize,
    stream_state: &mut StreamState<'_>,
) -> Result<(), Box<refusal::RefusalEnvelope>> {
    use std::io::Write;

    if pending_records.is_empty() {
        return Ok(());
    }

    // Keep memory bounded while still honoring deterministic ordered output.
    let processed_records =
        pipeline::parallel::process_indexed_in_parallel(pending_records, jobs, |(_, pending)| {
            process_record(pending, algorithm)
        });

    for processed_record in processed_records {
        let processed_record = processed_record?;

        if let Some(warning_event) = processed_record.warning_event.as_ref() {
            if stream_state.progress_enabled {
                let _ = progress::write_warning(stream_state.stderr, warning_event);
            } else {
                let _ = writeln!(
                    stream_state.stderr,
                    "hash: warning: {}: {}",
                    warning_event.path, warning_event.message
                );
            }
        }

        emit_processed_record(&processed_record.record, stream_state)?;

        if processed_record.skipped {
            *stream_state.any_skipped = true;
        }
    }

    Ok(())
}

fn process_record(
    pending: PendingRecord,
    algorithm: cli::Algorithm,
) -> Result<ProcessedRecord, Box<refusal::RefusalEnvelope>> {
    let PendingRecord {
        line_number,
        record,
    } = pending;

    let Some(record_obj) = record.as_object() else {
        return Err(Box::new(refusal::RefusalEnvelope::from_code(
            refusal::RefusalCode::BadInput,
            serde_json::json!({
                "line": line_number,
                "error": "record must be a JSON object"
            }),
        )));
    };

    if pipeline::enricher::is_skipped(record_obj) {
        return Ok(ProcessedRecord {
            record: pipeline::enricher::process_skipped_record(record),
            warning_event: None,
            skipped: true,
        });
    }

    let path_str = record
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            Box::new(refusal::RefusalEnvelope::bad_input_missing_field(
                line_number,
                "path",
            ))
        })?
        .to_owned();

    match hash::hash_file(std::path::Path::new(&path_str), algorithm) {
        Ok(file_hash) => Ok(ProcessedRecord {
            record: pipeline::enricher::process_hashed_record(
                record,
                file_hash,
                algorithm.prefix(),
            ),
            warning_event: None,
            skipped: false,
        }),
        Err(io_err) => {
            let warning_message = format!("skipped: {io_err}");

            Ok(ProcessedRecord {
                record: pipeline::enricher::process_io_failed_record(
                    record,
                    &path_str,
                    &io_err.to_string(),
                ),
                warning_event: Some(progress::WarningEvent::new(&path_str, &warning_message)),
                skipped: true,
            })
        }
    }
}

fn emit_processed_record(
    record: &Value,
    stream_state: &mut StreamState<'_>,
) -> Result<(), Box<refusal::RefusalEnvelope>> {
    use std::io::Write;

    let mut rendered = Vec::new();
    output::jsonl::write_json_line(&mut rendered, record)
        .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;
    stream_state
        .stdout
        .write_all(&rendered)
        .map_err(|err| Box::new(refusal::RefusalEnvelope::io_error(err.to_string())))?;
    stream_state.output_hasher.update(&rendered);

    *stream_state.processed += 1;
    if stream_state.progress_enabled {
        let elapsed_ms = u64::try_from(stream_state.progress_started_at.elapsed().as_millis())
            .unwrap_or(u64::MAX);
        let progress_event = progress::ProgressEvent::new(
            *stream_state.processed,
            *stream_state.processed,
            elapsed_ms,
        );
        let _ = progress::write_progress(stream_state.stderr, &progress_event);
    }

    Ok(())
}

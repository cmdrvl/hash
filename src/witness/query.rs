use super::ledger::default_witness_path;
use super::record::WitnessRecord;
use chrono::{DateTime, Utc};
use std::fs;
use std::io::BufRead;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WitnessQuery {
    pub tool: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub outcome: Option<String>,
    pub input_hash: Option<String>,
    pub limit: Option<usize>,
}

pub fn load_witness_records() -> Result<Vec<WitnessRecord>, std::io::Error> {
    let witness_path = default_witness_path();

    if !witness_path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(witness_path)?;
    let reader = std::io::BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<WitnessRecord>(&line) {
            Ok(record) => records.push(record),
            Err(_) => continue, // Skip invalid records
        }
    }

    Ok(records)
}

pub fn filter_records<'a>(
    records: &'a [WitnessRecord],
    query: &WitnessQuery,
) -> Vec<&'a WitnessRecord> {
    let since_bound = query.since.as_deref().and_then(parse_time_bound);
    let until_bound = query.until.as_deref().and_then(parse_time_bound);

    let mut filtered: Vec<&WitnessRecord> = records
        .iter()
        .filter(|record| match &query.tool {
            Some(tool) => &record.tool == tool,
            None => true,
        })
        .filter(|record| match &query.outcome {
            Some(outcome) => &record.outcome == outcome,
            None => true,
        })
        .filter(|record| within_bounds(record, since_bound.as_ref(), until_bound.as_ref()))
        .filter(|record| match &query.input_hash {
            Some(hash_filter) => {
                if let Some(ref output_hash) = record.output_hash {
                    output_hash.contains(hash_filter)
                } else {
                    false
                }
            }
            None => true,
        })
        .collect();

    // Sort by most recent first (assuming records are appended chronologically)
    filtered.reverse();

    // Apply limit if specified
    if let Some(limit) = query.limit {
        filtered.truncate(limit);
    }

    filtered
}

fn parse_time_bound(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn within_bounds(
    record: &WitnessRecord,
    since: Option<&DateTime<Utc>>,
    until: Option<&DateTime<Utc>>,
) -> bool {
    if since.is_none() && until.is_none() {
        return true;
    }

    let Some(ts_raw) = record.ts.as_deref() else {
        return false;
    };
    let Some(record_ts) = parse_time_bound(ts_raw) else {
        return false;
    };

    if let Some(since_bound) = since
        && record_ts < *since_bound
    {
        return false;
    }

    if let Some(until_bound) = until
        && record_ts > *until_bound
    {
        return false;
    }

    true
}

pub fn handle_witness_query(action: &crate::cli::WitnessAction) -> Result<u8, String> {
    use crate::cli::WitnessAction;

    match action {
        WitnessAction::Query {
            tool,
            since,
            until,
            outcome,
            input_hash,
            limit,
            json,
        } => {
            let query = WitnessQuery {
                tool: tool.clone(),
                since: since.clone(),
                until: until.clone(),
                outcome: outcome.clone(),
                input_hash: input_hash.clone(),
                limit: *limit,
            };

            let records = load_witness_records().map_err(|e| e.to_string())?;
            let filtered = filter_records(&records, &query);

            if *json {
                let json_output = serde_json::to_string(&filtered).map_err(|e| e.to_string())?;
                println!("{}", json_output);
            } else {
                if filtered.is_empty() {
                    println!("No matching witness records");
                } else {
                    for record in &filtered {
                        println!(
                            "{} {} {} (exit: {})",
                            record.tool, record.outcome, record.version, record.exit_code
                        );
                    }
                }
            }

            Ok(if filtered.is_empty() {
                crate::cli::exit_code(crate::cli::Outcome::Partial)
            } else {
                crate::cli::exit_code(crate::cli::Outcome::AllHashed)
            })
        }

        WitnessAction::Last { json } => {
            let records = load_witness_records().map_err(|e| e.to_string())?;
            let last_record = records.last();

            if *json {
                let json_output = serde_json::to_string(&last_record).map_err(|e| e.to_string())?;
                println!("{}", json_output);
            } else {
                match last_record {
                    Some(record) => {
                        println!(
                            "{} {} {} (exit: {})",
                            record.tool, record.outcome, record.version, record.exit_code
                        );
                    }
                    None => {
                        println!("No witness records found");
                    }
                }
            }

            Ok(if last_record.is_some() {
                crate::cli::exit_code(crate::cli::Outcome::AllHashed)
            } else {
                crate::cli::exit_code(crate::cli::Outcome::Partial)
            })
        }

        WitnessAction::Count {
            tool,
            since,
            until,
            outcome,
            input_hash,
            json,
        } => {
            let query = WitnessQuery {
                tool: tool.clone(),
                since: since.clone(),
                until: until.clone(),
                outcome: outcome.clone(),
                input_hash: input_hash.clone(),
                limit: None,
            };

            let records = load_witness_records().map_err(|e| e.to_string())?;
            let filtered = filter_records(&records, &query);

            let count = filtered.len();
            if *json {
                println!("{}", serde_json::json!({ "count": count }));
            } else {
                println!("{count}");
            }

            Ok(if count > 0 {
                crate::cli::exit_code(crate::cli::Outcome::AllHashed)
            } else {
                crate::cli::exit_code(crate::cli::Outcome::Partial)
            })
        }
    }
}

use super::record::{WitnessRecord, canonical_json};
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn default_witness_path() -> PathBuf {
    if let Ok(path) = env::var("EPISTEMIC_WITNESS")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }

    if let Ok(home) = env::var("HOME")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home).join(".epistemic/witness.jsonl");
    }

    if let Ok(home) = env::var("USERPROFILE")
        && !home.trim().is_empty()
    {
        return PathBuf::from(home).join(".epistemic/witness.jsonl");
    }

    PathBuf::from(".epistemic/witness.jsonl")
}

pub fn append_record(path: &Path, record: &WitnessRecord) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(canonical_json(record).as_bytes())?;
    file.write_all(b"\n")
}

pub fn append_default_record(record: &WitnessRecord) -> io::Result<PathBuf> {
    let path = default_witness_path();
    append_record(&path, record)?;
    Ok(path)
}

pub fn last_record_id(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let mut last_non_empty = None;
    for line in std::io::BufRead::lines(reader).map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            last_non_empty = Some(trimmed.to_owned());
        }
    }

    let last = last_non_empty?;
    let value: serde_json::Value = serde_json::from_str(&last).ok()?;
    value.get("id")?.as_str().map(ToOwned::to_owned)
}

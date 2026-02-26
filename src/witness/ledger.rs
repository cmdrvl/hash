use super::record::WitnessRecord;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn default_witness_path() -> PathBuf {
    if let Ok(path) = env::var("EPISTEMIC_WITNESS")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    if let Ok(home) = env::var("HOME")
        && !home.is_empty()
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
    serde_json::to_writer(&mut file, record).map_err(io::Error::other)?;
    file.write_all(b"\n")
}

pub fn append_default_record(record: &WitnessRecord) -> io::Result<PathBuf> {
    let path = default_witness_path();
    append_record(&path, record)?;
    Ok(path)
}

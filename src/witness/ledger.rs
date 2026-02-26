use super::record::WitnessRecord;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

pub fn append_record(path: &Path, record: &WitnessRecord) -> io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, record).map_err(io::Error::other)?;
    file.write_all(b"\n")
}

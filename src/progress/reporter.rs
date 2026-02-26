use serde::Serialize;
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProgressEvent {
    pub r#type: &'static str,
    pub tool: &'static str,
    pub processed: usize,
    pub total: usize,
}

impl ProgressEvent {
    pub fn new(processed: usize, total: usize) -> Self {
        Self {
            r#type: "progress",
            tool: "hash",
            processed,
            total,
        }
    }
}

pub fn write_progress<W: Write>(writer: &mut W, event: &ProgressEvent) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, event).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

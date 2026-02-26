use serde::Serialize;
use std::io::{self, Write};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProgressEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub tool: &'static str,
    pub processed: usize,
    pub total: usize,
    pub percent: f64,
    pub elapsed_ms: u64,
}

impl ProgressEvent {
    pub fn new(processed: usize, total: usize, elapsed_ms: u64) -> Self {
        let percent = if total == 0 {
            0.0
        } else {
            (processed as f64 / total as f64) * 100.0
        };

        Self {
            event_type: "progress",
            tool: "hash",
            processed,
            total,
            percent,
            elapsed_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WarningEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub tool: &'static str,
    pub path: String,
    pub message: String,
}

impl WarningEvent {
    pub fn new(path: &str, message: &str) -> Self {
        Self {
            event_type: "warning",
            tool: "hash",
            path: path.to_owned(),
            message: message.to_owned(),
        }
    }
}

pub fn write_progress<W: Write>(writer: &mut W, event: &ProgressEvent) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, event).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

pub fn write_warning<W: Write>(writer: &mut W, event: &WarningEvent) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, event).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

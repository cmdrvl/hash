use serde_json::Value;
use std::io::{self, BufRead};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLine {
    pub line_number: usize,
    pub record: Value,
}

pub fn read_line<R: BufRead>(reader: &mut R, buffer: &mut String) -> io::Result<usize> {
    buffer.clear();
    reader.read_line(buffer)
}

pub fn parse_json_line(line: &str, line_number: usize) -> Result<ParsedLine, serde_json::Error> {
    let record = serde_json::from_str::<Value>(line)?;
    Ok(ParsedLine {
        line_number,
        record,
    })
}

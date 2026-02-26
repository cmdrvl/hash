use serde_json::Value;
use std::io::{self, BufRead};

use crate::refusal::{RefusalCode, RefusalEnvelope};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLine {
    pub line_number: usize,
    pub record: Value,
}

pub fn read_line<R: BufRead>(reader: &mut R, buffer: &mut String) -> io::Result<usize> {
    buffer.clear();
    reader.read_line(buffer)
}

pub fn parse_json_line(line: &str, line_number: usize) -> Result<ParsedLine, Box<RefusalEnvelope>> {
    let record = serde_json::from_str::<Value>(line).map_err(|error| {
        Box::new(RefusalEnvelope::bad_input_parse_error(
            line_number,
            error.to_string(),
        ))
    })?;
    ensure_required_fields(&record, line_number)?;

    Ok(ParsedLine {
        line_number,
        record,
    })
}

fn ensure_required_fields(record: &Value, line_number: usize) -> Result<(), Box<RefusalEnvelope>> {
    let Some(object) = record.as_object() else {
        return Err(Box::new(RefusalEnvelope::from_code(
            RefusalCode::BadInput,
            serde_json::json!({
                "line": line_number,
                "error": "record must be a JSON object"
            }),
        )));
    };

    for required_field in ["path", "version"] {
        if object
            .get(required_field)
            .is_none_or(serde_json::Value::is_null)
        {
            return Err(Box::new(RefusalEnvelope::bad_input_missing_field(
                line_number,
                required_field,
            )));
        }
    }

    Ok(())
}

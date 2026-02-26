use super::codes::RefusalCode;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct RefusalEnvelope {
    pub version: &'static str,
    pub outcome: &'static str,
    pub refusal: Refusal,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
    pub detail: Value,
    pub next_command: Option<String>,
}

impl RefusalEnvelope {
    pub fn new(code: RefusalCode, message: impl Into<String>, detail: Value) -> Self {
        Self {
            version: "hash.v0",
            outcome: "REFUSAL",
            refusal: Refusal {
                code: code.as_str(),
                message: message.into(),
                detail,
                next_command: None,
            },
        }
    }

    pub fn from_code(code: RefusalCode, detail: Value) -> Self {
        Self::new(code, code.default_message(), detail)
    }

    pub fn bad_input_parse_error(line: usize, error: impl Into<String>) -> Self {
        Self::from_code(
            RefusalCode::BadInput,
            json!({ "line": line, "error": error.into() }),
        )
    }

    pub fn bad_input_missing_field(line: usize, field: impl Into<String>) -> Self {
        Self::from_code(
            RefusalCode::BadInput,
            json!({ "line": line, "missing_field": field.into() }),
        )
    }

    pub fn io_error(error: impl Into<String>) -> Self {
        Self::from_code(RefusalCode::Io, json!({ "error": error.into() }))
    }

    pub fn with_next_command(mut self, command: impl Into<String>) -> Self {
        self.refusal.next_command = Some(command.into());
        self
    }

    pub fn to_value(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }
}

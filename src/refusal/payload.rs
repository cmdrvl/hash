use super::codes::RefusalCode;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RefusalEnvelope {
    pub version: &'static str,
    pub outcome: &'static str,
    pub refusal: Refusal,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
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
}

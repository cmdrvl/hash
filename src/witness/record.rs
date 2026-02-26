use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessRecord {
    pub version: String,
    pub tool: String,
    pub outcome: String,
    pub exit_code: u8,
    pub output_hash: Option<String>,
    pub params: Map<String, Value>,
}

impl WitnessRecord {
    pub fn new(tool: impl Into<String>, outcome: impl Into<String>, exit_code: u8) -> Self {
        Self {
            version: "witness.v0".to_owned(),
            tool: tool.into(),
            outcome: outcome.into(),
            exit_code,
            output_hash: None,
            params: Map::new(),
        }
    }

    pub fn from_run(
        outcome: impl Into<String>,
        exit_code: u8,
        params: Map<String, Value>,
        output_bytes: &[u8],
    ) -> Self {
        let mut record = Self::new("hash", outcome, exit_code);
        record.params = params;
        record.output_hash = Some(format!("blake3:{}", blake3::hash(output_bytes).to_hex()));
        record
    }
}

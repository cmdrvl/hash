use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessInput {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessRecord {
    pub version: String,
    pub tool: String,
    pub outcome: String,
    pub exit_code: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<WitnessInput>,
    pub output_hash: Option<String>,
    pub ts: Option<String>,
    pub params: Map<String, Value>,
}

impl WitnessRecord {
    pub fn input(
        path: impl Into<String>,
        hash: Option<String>,
        bytes: Option<u64>,
    ) -> WitnessInput {
        WitnessInput {
            path: path.into(),
            hash,
            bytes,
        }
    }

    pub fn new(tool: impl Into<String>, outcome: impl Into<String>, exit_code: u8) -> Self {
        Self {
            version: "witness.v0".to_owned(),
            tool: tool.into(),
            outcome: outcome.into(),
            exit_code,
            inputs: Vec::new(),
            output_hash: None,
            ts: None,
            params: Map::new(),
        }
    }

    pub fn from_run(
        inputs: Vec<WitnessInput>,
        outcome: impl Into<String>,
        exit_code: u8,
        params: Map<String, Value>,
        output_hash: String,
    ) -> Self {
        let mut record = Self::new("hash", outcome, exit_code);
        record.inputs = inputs;
        record.params = params;
        record.output_hash = Some(output_hash);
        record.ts = Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        record
    }
}

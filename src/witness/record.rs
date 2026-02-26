use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitnessRecord {
    pub tool: String,
    pub outcome: String,
    pub exit_code: u8,
    pub params: Map<String, Value>,
}

impl WitnessRecord {
    pub fn new(tool: impl Into<String>, outcome: impl Into<String>, exit_code: u8) -> Self {
        Self {
            tool: tool.into(),
            outcome: outcome.into(),
            exit_code,
            params: Map::new(),
        }
    }
}

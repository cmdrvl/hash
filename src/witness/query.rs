use super::record::WitnessRecord;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WitnessQuery {
    pub tool: Option<String>,
    pub outcome: Option<String>,
}

pub fn filter_records<'a>(
    records: &'a [WitnessRecord],
    query: &WitnessQuery,
) -> Vec<&'a WitnessRecord> {
    records
        .iter()
        .filter(|record| match &query.tool {
            Some(tool) => &record.tool == tool,
            None => true,
        })
        .filter(|record| match &query.outcome {
            Some(outcome) => &record.outcome == outcome,
            None => true,
        })
        .collect()
}

pub fn handle_witness_query(action: &crate::cli::WitnessAction) -> Result<u8, String> {
    let _ = action;
    Ok(0)
}

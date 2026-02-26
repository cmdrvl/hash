#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    AllHashed, // exit 0
    Partial,   // exit 1
    Refusal,   // exit 2
}

impl Outcome {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::AllHashed => 0,
            Self::Partial => 1,
            Self::Refusal => 2,
        }
    }
}

pub fn exit_code(outcome: Outcome) -> u8 {
    outcome.exit_code()
}

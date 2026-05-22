use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    BadInput,
    GuardPreflight,
    Io,
}

impl RefusalCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadInput => "E_BAD_INPUT",
            Self::GuardPreflight => "E_GUARD_PREFLIGHT",
            Self::Io => "E_IO",
        }
    }

    pub fn default_message(self) -> &'static str {
        match self {
            Self::BadInput => "Input is not valid JSONL or missing required fields",
            Self::GuardPreflight => {
                "Required Claude PreToolUse guard hooks are missing or unhealthy"
            }
            Self::Io => "Cannot read input/output stream",
        }
    }
}

impl fmt::Display for RefusalCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

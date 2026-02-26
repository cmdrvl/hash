use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Sha256,
    Blake3,
}

impl Algorithm {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Blake3 => "blake3",
        }
    }

    pub fn format_bytes_hash(&self, digest_hex: &str) -> String {
        format!("{}:{}", self.prefix(), digest_hex.to_ascii_lowercase())
    }
}

impl FromStr for Algorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("sha256") {
            Ok(Self::Sha256)
        } else if s.eq_ignore_ascii_case("blake3") {
            Ok(Self::Blake3)
        } else {
            Err(format!(
                "Invalid algorithm '{s}'. Expected one of: sha256, blake3"
            ))
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.prefix())
    }
}

#[cfg(test)]
mod tests {
    use super::Algorithm;
    use std::str::FromStr;

    #[test]
    fn parses_case_insensitive_values() {
        assert_eq!(
            Algorithm::from_str("sha256").expect("parse sha256"),
            Algorithm::Sha256
        );
        assert_eq!(
            Algorithm::from_str("SHA256").expect("parse SHA256"),
            Algorithm::Sha256
        );
        assert_eq!(
            Algorithm::from_str("BLAKE3").expect("parse BLAKE3"),
            Algorithm::Blake3
        );
    }

    #[test]
    fn rejects_invalid_algorithm_names() {
        let error = Algorithm::from_str("md5").expect_err("md5 must be rejected");
        assert!(error.contains("sha256"));
        assert!(error.contains("blake3"));
    }

    #[test]
    fn formats_prefixed_hash_with_lowercase_hex() {
        let bytes_hash = Algorithm::Blake3.format_bytes_hash("ABCDEF1234");
        assert_eq!(bytes_hash, "blake3:abcdef1234");
    }
}

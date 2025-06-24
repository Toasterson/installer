use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Error type for OCI digest operations
#[derive(Debug, Error)]
pub enum DigestError {
    #[error("Invalid digest format: {0}")]
    InvalidFormat(String),
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/// Represents an OCI content digest
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OciDigest {
    algorithm: String,
    hex: String,
}

impl OciDigest {
    /// Create a new OciDigest with the given algorithm and hex value
    pub fn new(algorithm: String, hex: String) -> Self {
        Self { algorithm, hex }
    }

    /// Get the algorithm part of the digest
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// Get the hex part of the digest
    pub fn hex(&self) -> &str {
        &self.hex
    }
}

impl fmt::Display for OciDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.hex)
    }
}

impl FromStr for OciDigest {
    type Err = DigestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(DigestError::InvalidFormat(s.to_string()));
        }

        let algorithm = parts[0].to_string();
        let hex = parts[1].to_string();

        // Validate algorithm (currently only sha256 is supported)
        if algorithm != "sha256" {
            return Err(DigestError::UnsupportedAlgorithm(algorithm));
        }

        // Validate hex is valid hexadecimal
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DigestError::InvalidFormat(s.to_string()));
        }

        Ok(OciDigest { algorithm, hex })
    }
}

impl serde::Serialize for OciDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for OciDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        OciDigest::from_str(&s).map_err(serde::de::Error::custom)
    }
}
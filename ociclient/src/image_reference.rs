use std::str::FromStr;
use thiserror::Error;

/// Error type for image reference parsing
#[derive(Debug, Error)]
pub enum ImageReferenceError {
    #[error("Invalid image reference format: {0}")]
    InvalidFormat(String),
}

/// Represents an OCI image reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReference {
    /// Optional hostname (registry)
    pub hostname: Option<String>,
    /// Repository name
    pub name: String,
    /// Tag or digest
    pub tag: String,
}

impl ImageReference {
    /// Create a new ImageReference
    pub fn new(hostname: Option<String>, name: String, tag: String) -> Self {
        Self { hostname, name, tag }
    }
}

impl FromStr for ImageReference {
    type Err = ImageReferenceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse image reference in the format: [hostname/]name[:tag]
        let mut parts = s.split('/');
        
        // Check if we have a hostname
        let (hostname, name) = if s.contains('/') && (s.contains('.') || s.contains(':')) && !s.starts_with('/') {
            let hostname = parts.next().unwrap();
            let name = parts.collect::<Vec<&str>>().join("/");
            (Some(hostname.to_string()), name)
        } else {
            (None, s.to_string())
        };
        
        // Extract tag if present
        let (name, tag) = if name.contains(':') {
            let name_parts: Vec<&str> = name.split(':').collect();
            (name_parts[0].to_string(), name_parts[1].to_string())
        } else {
            (name, "latest".to_string())
        };
        
        Ok(ImageReference {
            hostname,
            name,
            tag,
        })
    }
}

impl ToString for ImageReference {
    fn to_string(&self) -> String {
        let prefix = if let Some(hostname) = &self.hostname {
            format!("{}/", hostname)
        } else {
            String::new()
        };
        
        format!("{}{}:{}", prefix, self.name, self.tag)
    }
}
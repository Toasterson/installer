use serde::{Deserialize, Serialize};

use crate::digest::OciDigest;

/// Represents a descriptor for a content blob in an OCI registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Descriptor {
    /// Media type of the referenced content
    pub media_type: String,
    /// Digest of the referenced content
    pub digest: OciDigest,
    /// Size of the referenced content in bytes
    pub size: usize,
}

/// Represents an OCI image manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageManifest {
    /// Schema version of the manifest
    pub schema_version: i32,
    /// Media type of the manifest
    pub media_type: String,
    /// Descriptor for the config blob
    pub config: Descriptor,
    /// Descriptors for the layer blobs
    pub layers: Vec<Descriptor>,
}
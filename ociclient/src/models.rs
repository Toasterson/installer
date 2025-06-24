use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Optional platform information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,
}

/// Represents platform information for a manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    /// Operating system
    pub os: String,
    /// CPU architecture
    pub architecture: String,
    /// Optional variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
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

/// Represents an OCI image manifest list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageManifestList {
    /// Schema version of the manifest list
    pub schema_version: i32,
    /// Media type of the manifest list
    pub media_type: String,
    /// List of manifests
    pub manifests: Vec<Descriptor>,
}

/// Represents an OCI artifact manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Schema version of the artifact manifest
    pub schema_version: i32,
    /// Media type of the artifact manifest
    pub media_type: String,
    /// Descriptor for the config blob
    pub config: Descriptor,
    /// Descriptors for the layer blobs
    pub layers: Vec<Descriptor>,
    /// Optional subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,
    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
}

/// Enum representing different types of OCI manifests
#[derive(Debug, Clone)]
pub enum ManifestVariant {
    /// Standard OCI image manifest
    Manifest(ImageManifest),
    /// OCI image manifest list
    List(ImageManifestList),
    /// OCI artifact manifest
    Artifact(ArtifactManifest),
}

/// Represents any OCI config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnyOciConfig {
    /// Optional architecture
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    /// Optional OS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    /// Optional config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    /// Optional rootfs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rootfs: Option<Rootfs>,
    /// Optional history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<HistoryEntry>>,
    /// Layer digests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<OciDigest>>,
}

impl AnyOciConfig {
    /// Get the layers from the config
    pub fn layers(&self) -> Vec<OciDigest> {
        self.layers.clone().unwrap_or_default()
    }
}

/// Represents rootfs information in an OCI config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rootfs {
    /// Type of the rootfs
    #[serde(rename = "type")]
    pub rootfs_type: String,
    /// Diff IDs
    pub diff_ids: Vec<String>,
}

/// Represents a history entry in an OCI config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Optional created timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Optional author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Optional created by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Optional comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Optional empty layer flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_layer: Option<bool>,
}

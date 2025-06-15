use std::env;
use std::fmt;
use std::path::PathBuf;

use config::{Config, Environment, File};
use serde::Deserialize;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub fs_root: Option<PathBuf>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Fs,
    S3,
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageBackend::Fs => write!(f, "fs"),
            StorageBackend::S3 => write!(f, "s3"),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());
        
        let config = Config::builder()
            // Start with default values
            .set_default("port", 8080)?
            .set_default("storage.backend", "fs")?
            .set_default("storage.fs_root", "./data")?
            
            // Add configuration from files
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            
            // Add environment variables with prefix IMGDEPOT_
            .add_source(Environment::with_prefix("IMGDEPOT").separator("_"))
            
            .build()
            .map_err(|e| AppError::Config(e.to_string()))?;
        
        // If in production mode and no explicit backend is set, default to S3
        let config = if run_mode == "production" {
            let backend = config.get_string("storage.backend").unwrap_or_else(|_| "s3".to_string());
            Config::builder()
                .add_source(config)
                .set_default("storage.backend", backend)?
                .build()
                .map_err(|e| AppError::Config(e.to_string()))?
        } else {
            config
        };
        
        config.try_deserialize()
            .map_err(|e| AppError::Config(e.to_string()))
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            storage: StorageConfig {
                backend: StorageBackend::Fs,
                fs_root: Some(PathBuf::from("./data")),
                s3_bucket: None,
                s3_region: None,
                s3_endpoint: None,
                s3_access_key: None,
                s3_secret_key: None,
            },
        }
    }
}
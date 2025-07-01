use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct InstallAdmConfig {
    pub boot_files_url: String,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("installadm")
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl InstallAdmConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());
        let mut builder = Config::builder()
            // Start with default values
            .set_default("boot_files_url", "https://dlc.openindiana.org/netboot/installer/current/boot_files.tar.gz")
                .map_err(|e| ConfigError::ConfigError(e.to_string()))?;

        // Add configuration from files in /etc/installadm.d directory
        if let Ok(entries) = fs::read_dir("/etc/installadm.d") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if ext_str == "toml" {
                            builder = builder.add_source(File::from(path).format(FileFormat::Toml).required(false));
                        } else if ext_str == "yaml" || ext_str == "yml" {
                            builder = builder.add_source(File::from(path).format(FileFormat::Yaml).required(false));
                        }
                    }
                }
            }
        }

        // Add configuration from legacy locations for backward compatibility
        builder = builder
            .add_source(File::with_name("/etc/installadm/config").required(false))
            .add_source(File::with_name(&format!("/etc/installadm/config.{}", run_mode)).required(false))
            .add_source(File::with_name("~/.config/installadm/config").required(false))
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false));

        // Add environment variables with prefix INSTALLADM_
        builder = builder.add_source(Environment::with_prefix("INSTALLADM").separator("_"));

        let config = builder.build()
            .map_err(|e| ConfigError::ConfigError(e.to_string()))?;

        let mut config_obj: Self = config.try_deserialize()
            .map_err(|e| ConfigError::ConfigError(e.to_string()))?;

        // Check if boot_files_url is a local file path and convert it to a file:// URL if needed
        if !config_obj.boot_files_url.starts_with("http://") && 
           !config_obj.boot_files_url.starts_with("https://") && 
           !config_obj.boot_files_url.starts_with("file://") {
            let path = Path::new(&config_obj.boot_files_url);
            if path.exists() {
                // Convert to absolute path if it's relative
                let abs_path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    std::env::current_dir()
                        .map_err(|e| ConfigError::ConfigError(format!("Failed to get current directory: {}", e)))?
                        .join(path)
                };

                // Convert to file:// URL
                config_obj.boot_files_url = format!("file://{}", abs_path.display());
            }
        }

        Ok(config_obj)
    }
}

impl Default for InstallAdmConfig {
    fn default() -> Self {
        Self {
            boot_files_url: "https://dlc.openindiana.org/netboot/installer/current/boot_files.tar.gz".to_string(),
            cache_dir: default_cache_dir(),
        }
    }
}

use anyhow::{Context, Result};
use std::time::Duration;
use tracing::{debug, trace};

/// Check if a metadata service is available
pub async fn check_metadata_service(
    url: &str,
    headers: Option<Vec<(&str, &str)>>,
    timeout_seconds: u64,
) -> bool {
    debug!("Checking metadata service at: {}", url);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            trace!("Failed to create HTTP client: {}", e);
            return false;
        }
    };

    let mut request = client.get(url);

    // Add custom headers if provided
    if let Some(headers) = headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            debug!("Metadata service response: {}", status);
            status.is_success()
        }
        Err(e) => {
            trace!("Metadata service not available: {}", e);
            false
        }
    }
}

/// Fetch metadata from a URL with optional headers
pub async fn fetch_metadata(
    url: &str,
    headers: Option<Vec<(&str, &str)>>,
    timeout_seconds: u64,
) -> Result<String> {
    debug!("Fetching metadata from: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .build()
        .context("Failed to create HTTP client")?;

    let mut request = client.get(url);

    // Add custom headers if provided
    if let Some(headers) = headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("Failed to fetch from {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP request failed with status: {}",
            response.status()
        ));
    }

    response
        .text()
        .await
        .context("Failed to read response body")
}

/// Fetch metadata as JSON
pub async fn fetch_metadata_json(
    url: &str,
    headers: Option<Vec<(&str, &str)>>,
    timeout_seconds: u64,
) -> Result<serde_json::Value> {
    let text = fetch_metadata(url, headers, timeout_seconds).await?;
    serde_json::from_str(&text).context("Failed to parse JSON response")
}

/// Convert a netmask to CIDR prefix length
pub fn netmask_to_cidr(netmask: &str) -> Result<u8> {
    let parts: Vec<&str> = netmask.split('.').collect();
    if parts.len() != 4 {
        return Err(anyhow::anyhow!("Invalid netmask format: {}", netmask));
    }

    let mut prefix_len = 0u8;
    for part in parts {
        let octet = part
            .parse::<u8>()
            .with_context(|| format!("Invalid octet in netmask: {}", part))?;
        prefix_len += octet.count_ones() as u8;
    }

    Ok(prefix_len)
}

/// Convert CIDR notation to netmask
pub fn cidr_to_netmask(prefix_len: u8) -> Result<String> {
    if prefix_len > 32 {
        return Err(anyhow::anyhow!(
            "Invalid CIDR prefix length: {}",
            prefix_len
        ));
    }

    let mask = !((1u32 << (32 - prefix_len)) - 1);
    let octets = [
        (mask >> 24) & 0xff,
        (mask >> 16) & 0xff,
        (mask >> 8) & 0xff,
        mask & 0xff,
    ];

    Ok(format!(
        "{}.{}.{}.{}",
        octets[0], octets[1], octets[2], octets[3]
    ))
}

/// Parse an IP address with optional CIDR notation
pub fn parse_ip_cidr(address: &str) -> Result<(String, Option<u8>)> {
    if let Some(pos) = address.find('/') {
        let ip = address[..pos].to_string();
        let prefix = address[pos + 1..]
            .parse::<u8>()
            .with_context(|| format!("Invalid CIDR prefix in: {}", address))?;
        Ok((ip, Some(prefix)))
    } else {
        Ok((address.to_string(), None))
    }
}

/// Normalize a MAC address to lowercase with colons
pub fn normalize_mac_address(mac: &str) -> String {
    mac.to_lowercase()
        .replace('-', ":")
        .chars()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(":")
}

/// Check if a path is a block device
#[cfg(unix)]
pub fn is_block_device(path: &std::path::Path) -> bool {
    use std::os::unix::fs::FileTypeExt;

    match std::fs::metadata(path) {
        Ok(metadata) => metadata.file_type().is_block_device(),
        Err(_) => false,
    }
}

#[cfg(not(unix))]
pub fn is_block_device(_path: &std::path::Path) -> bool {
    false
}

/// Mount a filesystem
#[cfg(unix)]
pub async fn mount_filesystem(
    device: &std::path::Path,
    mount_point: &std::path::Path,
    fs_type: Option<&str>,
) -> Result<()> {
    use std::process::Command;

    // Ensure mount point exists
    tokio::fs::create_dir_all(mount_point)
        .await
        .with_context(|| format!("Failed to create mount point: {:?}", mount_point))?;

    let mut cmd = Command::new("mount");

    if let Some(fs) = fs_type {
        cmd.arg("-t").arg(fs);
    }

    cmd.arg(device).arg(mount_point);

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute mount command"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to mount {:?} to {:?}: {}",
            device,
            mount_point,
            stderr
        ));
    }

    Ok(())
}

/// Unmount a filesystem
#[cfg(unix)]
pub async fn unmount_filesystem(mount_point: &std::path::Path) -> Result<()> {
    use std::process::Command;

    let output = Command::new("umount")
        .arg(mount_point)
        .output()
        .with_context(|| format!("Failed to execute umount command"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to unmount {:?}: {}",
            mount_point,
            stderr
        ));
    }

    Ok(())
}

#[cfg(not(unix))]
pub async fn mount_filesystem(
    _device: &std::path::Path,
    _mount_point: &std::path::Path,
    _fs_type: Option<&str>,
) -> Result<()> {
    Err(anyhow::anyhow!("Mounting not supported on this platform"))
}

#[cfg(not(unix))]
pub async fn unmount_filesystem(_mount_point: &std::path::Path) -> Result<()> {
    Err(anyhow::anyhow!("Unmounting not supported on this platform"))
}

/// Find a device by label
#[cfg(target_os = "linux")]
pub async fn find_device_by_label(label: &str) -> Option<std::path::PathBuf> {
    let path = format!("/dev/disk/by-label/{}", label);
    let device_path = std::path::Path::new(&path);

    if device_path.exists() {
        // Resolve symlink to actual device
        if let Ok(resolved) = tokio::fs::read_link(device_path).await {
            return Some(resolved);
        }
        return Some(device_path.to_path_buf());
    }

    // Alternative: use blkid
    if let Ok(output) = std::process::Command::new("blkid")
        .args(&["-o", "device", "-t", &format!("LABEL={}", label)])
        .output()
    {
        if output.status.success() {
            let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !device.is_empty() {
                return Some(std::path::PathBuf::from(device));
            }
        }
    }

    None
}

#[cfg(not(target_os = "linux"))]
pub async fn find_device_by_label(_label: &str) -> Option<std::path::PathBuf> {
    // Platform-specific implementation needed
    None
}

/// Decode base64 data
pub fn decode_base64(data: &str) -> Result<Vec<u8>> {
    base64::decode(data).context("Failed to decode base64 data")
}

/// Encode data as base64
pub fn encode_base64(data: &[u8]) -> String {
    base64::encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_netmask_to_cidr() {
        assert_eq!(netmask_to_cidr("255.255.255.0").unwrap(), 24);
        assert_eq!(netmask_to_cidr("255.255.0.0").unwrap(), 16);
        assert_eq!(netmask_to_cidr("255.0.0.0").unwrap(), 8);
        assert_eq!(netmask_to_cidr("255.255.255.128").unwrap(), 25);
    }

    #[test]
    fn test_cidr_to_netmask() {
        assert_eq!(cidr_to_netmask(24).unwrap(), "255.255.255.0");
        assert_eq!(cidr_to_netmask(16).unwrap(), "255.255.0.0");
        assert_eq!(cidr_to_netmask(8).unwrap(), "255.0.0.0");
        assert_eq!(cidr_to_netmask(25).unwrap(), "255.255.255.128");
    }

    #[test]
    fn test_parse_ip_cidr() {
        let (ip, cidr) = parse_ip_cidr("192.168.1.10/24").unwrap();
        assert_eq!(ip, "192.168.1.10");
        assert_eq!(cidr, Some(24));

        let (ip, cidr) = parse_ip_cidr("10.0.0.1").unwrap();
        assert_eq!(ip, "10.0.0.1");
        assert_eq!(cidr, None);
    }

    #[test]
    fn test_normalize_mac_address() {
        assert_eq!(
            normalize_mac_address("AA:BB:CC:DD:EE:FF"),
            "aa:bb:cc:dd:ee:ff"
        );
        assert_eq!(
            normalize_mac_address("aa-bb-cc-dd-ee-ff"),
            "aa:bb:cc:dd:ee:ff"
        );
    }
}

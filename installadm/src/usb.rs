use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use libarchive::archive::{ReadCompression, ReadFormat};
use ociclient::{Client as OciClient, ImageReference, ManifestVariant};
use reqwest::Client;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{absolute, Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;
use url::Url;
use which::which;
use serde_json::json;

use crate::config::InstallAdmConfig;
use crate::Error;

// For testrun mode
use std::fs::OpenOptions;

// Platform-specific disk utilities
#[cfg(target_os = "linux")]
pub const DISK_UTIL: &str = "parted";
#[cfg(target_os = "macos")]
pub const DISK_UTIL: &str = "diskutil";
#[cfg(target_os = "windows")]
pub const DISK_UTIL: &str = "diskpart";
#[cfg(target_os = "illumos")]
pub const DISK_UTIL: &str = "format";

// Platform-specific device path prefixes
#[cfg(target_os = "linux")]
pub const DEVICE_PREFIX: &str = "/dev/";
#[cfg(target_os = "macos")]
pub const DEVICE_PREFIX: &str = "/dev/";
#[cfg(target_os = "windows")]
pub const DEVICE_PREFIX: &str = "\\\\.\\";
#[cfg(target_os = "illumos")]
pub const DEVICE_PREFIX: &str = "/dev/dsk/";

/// Create a bootable USB stick with EFI boot files
pub async fn create_bootable_usb(
    device: &str,
    oci_image: Option<&str>,
    size_gb: u64,
    assets_url: Option<&str>,
) -> Result<(), Error> {
    // Load configuration
    let config = InstallAdmConfig::load()
        .map_err(|e| Error::CommandError(format!("Failed to load configuration: {}", e)))?;

    // Check if required tools are available
    check_required_tools()?;

    // Normalize device path
    let device_path = normalize_device_path(device)?;
    println!("Creating bootable USB on device: {}", device_path);

    // Ensure the device exists, creating an empty file if it doesn't
    // Use the specified size or a default size if creating a new file
    // This also sets up a loop device if the target is an image file
    let (actual_device, using_loop) = ensure_device_exists(&device_path, size_gb)?;

    // Create FAT32 partition
    create_fat32_partition(&actual_device, size_gb)?;

    // Get mount point
    let mount_point = get_mount_point(&actual_device)?;
    println!("USB device mounted at: {}", mount_point.display());

    // Download and extract boot files
    download_and_extract_boot_files(&config.boot_files_url, &mount_point).await?;

    // Download additional assets if specified
    if let Some(url) = assets_url {
        download_and_extract_assets(url, &mount_point).await?;
    }

    // Download OCI image if specified
    if let Some(image) = oci_image {
        download_oci_image(image, &mount_point).await?;
    }

    // Clean up loop device if we created one
    if using_loop {
        cleanup_loop_device(&actual_device)?;
    }

    println!("Bootable USB created successfully!");
    Ok(())
}

/// Check if required tools are available
fn check_required_tools() -> Result<(), Error> {
    which(DISK_UTIL).map_err(|_| Error::ToolNotFound(DISK_UTIL.to_string()))?;

    // Platform-specific tools
    #[cfg(target_os = "linux")]
    {
        which("mkfs.fat").map_err(|_| Error::ToolNotFound("mkfs.fat".to_string()))?;
        which("mount").map_err(|_| Error::ToolNotFound("mount".to_string()))?;
        which("umount").map_err(|_| Error::ToolNotFound("umount".to_string()))?;
        which("losetup").map_err(|_| Error::ToolNotFound("losetup".to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        which("newfs_msdos").map_err(|_| Error::ToolNotFound("newfs_msdos".to_string()))?;
        which("hdiutil").map_err(|_| Error::ToolNotFound("hdiutil".to_string()))?;
    }

    #[cfg(target_os = "windows")]
    {
        which("format").map_err(|_| Error::ToolNotFound("format".to_string()))?;
    }

    #[cfg(target_os = "illumos")]
    {
        which("mkfs").map_err(|_| Error::ToolNotFound("mkfs".to_string()))?;
        which("mount").map_err(|_| Error::ToolNotFound("mount".to_string()))?;
        which("umount").map_err(|_| Error::ToolNotFound("umount".to_string()))?;
        which("lofiadm").map_err(|_| Error::ToolNotFound("lofiadm".to_string()))?;
    }

    Ok(())
}

/// Normalize device path based on platform
fn normalize_device_path(device: &str) -> Result<String, Error> {
    if device.starts_with(DEVICE_PREFIX) {
        Ok(device.to_string())
    } else {
        let p = Path::new(device);
        if p.is_absolute() {
            Ok(device.to_string())
        } else {
            Ok(absolute(p)?.to_string_lossy().to_string())
        }
    }
}

/// Ensure the device exists, creating an empty file if it doesn't
/// Returns a tuple with the actual device path to use (which might be a loop device) and a boolean
/// indicating whether a loop device was set up (true) or not (false)
fn ensure_device_exists(device_path: &str, size_gb: u64) -> Result<(String, bool), Error> {
    let path = Path::new(device_path);

    // Check if the path exists
    if !path.exists() {
        println!(
            "Device path does not exist. Creating empty file: {}",
            device_path
        );

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| Error::IoError(e))?;
            }
        }

        // Create an empty file with the specified size
        let file_size = size_gb * 1024 * 1024 * 1024; // Convert GB to bytes
        let mut file = File::create(path).map_err(|e| Error::IoError(e))?;

        // Set the file size by seeking to the desired size and writing a byte
        file.seek(SeekFrom::Start(file_size - 1))
            .map_err(|e| Error::IoError(e))?;
        file.write_all(&[0]).map_err(|e| Error::IoError(e))?;

        println!(
            "Created empty file of size {}GB at {}",
            size_gb, device_path
        );
    }

    // Check if this is a regular file (image file) or a block device
    let metadata = fs::metadata(path).map_err(|e| Error::IoError(e))?;

    if metadata.is_file() {
        // This is an image file, set up a loop device
        println!("Setting up loop device for image file: {}", device_path);
        let loop_device = setup_loop_device(device_path)?;
        println!("Using loop device: {}", loop_device);
        Ok((loop_device, true))
    } else {
        // This is a physical device, use it directly
        Ok((device_path.to_string(), false))
    }
}

/// Set up a loop device for an image file
fn setup_loop_device(file_path: &str) -> Result<String, Error> {
    #[cfg(target_os = "linux")]
    {
        // Check if losetup is available
        which("losetup").map_err(|_| Error::ToolNotFound("losetup".to_string()))?;

        // Set up a loop device
        let output = Command::new("losetup")
            .args(["--find", "--show", file_path])
            .output()
            .map_err(|e| Error::CommandError(format!("Failed to set up loop device: {}", e)))?;

        if !output.status.success() {
            return Err(Error::CommandError(format!(
                "Failed to set up loop device: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Get the loop device path
        let loop_device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if loop_device.is_empty() {
            return Err(Error::CommandError(
                "Failed to get loop device path".to_string(),
            ));
        }

        Ok(loop_device)
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use hdiutil to attach the disk image
        let output = Command::new("hdiutil")
            .args(["attach", "-nomount", file_path])
            .output()
            .map_err(|e| Error::CommandError(format!("Failed to attach disk image: {}", e)))?;

        if !output.status.success() {
            return Err(Error::CommandError(format!(
                "Failed to attach disk image: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse the output to get the device path
        let output_str = String::from_utf8_lossy(&output.stdout);
        let device_line = output_str
            .lines()
            .next()
            .ok_or_else(|| Error::CommandError("Failed to get device path".to_string()))?;

        let device_path = device_line
            .split_whitespace()
            .next()
            .ok_or_else(|| Error::CommandError("Failed to parse device path".to_string()))?
            .to_string();

        Ok(device_path)
    }

    #[cfg(target_os = "illumos")]
    {
        // On illumos, use lofiadm to add the file as a block device
        let output = Command::new("lofiadm")
            .args(["-a", file_path])
            .output()
            .map_err(|e| Error::CommandError(format!("Failed to add lofi device: {}", e)))?;

        if !output.status.success() {
            return Err(Error::CommandError(format!(
                "Failed to add lofi device: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse the output to get the device path
        let lofi_device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if lofi_device.is_empty() {
            // If no output, try to find the device
            let output = Command::new("lofiadm")
                .output()
                .map_err(|e| Error::CommandError(format!("Failed to list lofi devices: {}", e)))?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(file_path) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if !parts.is_empty() {
                        return Ok(parts[0].to_string());
                    }
                }
            }

            return Err(Error::CommandError(
                "Failed to get lofi device path".to_string(),
            ));
        }

        Ok(lofi_device)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "illumos")))]
    {
        Err(Error::CommandError(
            "Loop device setup not supported on this platform".to_string(),
        ))
    }
}

/// Clean up a loop device
fn cleanup_loop_device(device_path: &str) -> Result<(), Error> {
    println!("Cleaning up loop device: {}", device_path);

    #[cfg(target_os = "linux")]
    {
        // Check if losetup is available
        which("losetup").map_err(|_| Error::ToolNotFound("losetup".to_string()))?;

        // Detach the loop device
        let status = Command::new("losetup")
            .args(["-d", device_path])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to detach loop device: {}", e)))?;

        if !status.success() {
            return Err(Error::CommandError(
                "Failed to detach loop device".to_string(),
            ));
        }

        println!("Loop device detached successfully");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, use hdiutil to detach the disk image
        let status = Command::new("hdiutil")
            .args(["detach", device_path])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to detach disk image: {}", e)))?;

        if !status.success() {
            return Err(Error::CommandError(
                "Failed to detach disk image".to_string(),
            ));
        }

        println!("Disk image detached successfully");
        Ok(())
    }

    #[cfg(target_os = "illumos")]
    {
        // On illumos, use lofiadm to remove the lofi device
        let status = Command::new("lofiadm")
            .args(["-d", device_path])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to remove lofi device: {}", e)))?;

        if !status.success() {
            return Err(Error::CommandError(
                "Failed to remove lofi device".to_string(),
            ));
        }

        println!("Lofi device removed successfully");
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "illumos")))]
    {
        Err(Error::CommandError(
            "Loop device cleanup not supported on this platform".to_string(),
        ))
    }
}

/// Create a FAT32 partition on the device
fn create_fat32_partition(device: &str, size_gb: u64) -> Result<(), Error> {
    println!("Creating FAT32 partition of size {}GB...", size_gb);

    // Platform-specific partition creation
    #[cfg(target_os = "linux")]
    {
        // Clear partition table
        let status = Command::new(DISK_UTIL)
            .args(["--script", device, "mklabel", "gpt"])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to create partition table: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to create partition table".to_string(),
            ));
        }

        // Create partition
        let status = Command::new(DISK_UTIL)
            .args([
                "--script",
                device,
                "mkpart",
                "primary",
                "fat32",
                "1MiB",
                &format!("{}GB", size_gb),
            ])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to create partition: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to create partition".to_string(),
            ));
        }

        // Set boot flag
        let status = Command::new(DISK_UTIL)
            .args(["--script", device, "set", "1", "boot", "on"])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to set boot flag: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError("Failed to set boot flag".to_string()));
        }

        // Format partition
        let partition = format!("{}p1", device);

        let status = Command::new("mkfs.fat")
            .args(["-F", "32", &partition])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to format partition: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to format partition".to_string(),
            ));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Unmount disk if mounted
        let _ = Command::new(DISK_UTIL)
            .args(["unmountDisk", device])
            .status();

        // Partition disk
        let status = Command::new(DISK_UTIL)
            .args([
                "partitionDisk",
                device,
                "GPT",
                "FAT32",
                "EFI",
                &format!("{}g", size_gb),
            ])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to partition disk: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to partition disk".to_string(),
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Create a script file for diskpart
        let temp_dir = tempdir().map_err(|e| Error::IoError(e))?;
        let script_path = temp_dir.path().join("diskpart.txt");
        let mut script = File::create(&script_path).map_err(|e| Error::IoError(e))?;

        // Extract disk number from device path
        let disk_num = device
            .trim_start_matches(DEVICE_PREFIX)
            .trim_start_matches("PHYSICALDRIVE")
            .parse::<u32>()
            .map_err(|_| Error::PartitionError("Invalid device path".to_string()))?;

        // Write diskpart script
        writeln!(script, "select disk {}", disk_num).map_err(|e| Error::IoError(e))?;
        writeln!(script, "clean").map_err(|e| Error::IoError(e))?;
        writeln!(script, "convert gpt").map_err(|e| Error::IoError(e))?;
        writeln!(script, "create partition primary size={}", size_gb * 1024)
            .map_err(|e| Error::IoError(e))?;
        writeln!(script, "format quick fs=fat32 label=EFI").map_err(|e| Error::IoError(e))?;
        writeln!(script, "assign letter=Z").map_err(|e| Error::IoError(e))?;
        writeln!(script, "exit").map_err(|e| Error::IoError(e))?;

        // Run diskpart with script
        let status = Command::new(DISK_UTIL)
            .args(["/s", script_path.to_str().unwrap()])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to run diskpart: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to partition disk".to_string(),
            ));
        }
    }

    #[cfg(target_os = "illumos")]
    {
        // Create partition
        let status = Command::new(DISK_UTIL)
            .args(["-e", device])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to partition disk: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to partition disk".to_string(),
            ));
        }

        // Format partition
        let partition = format!("{}s0", device);
        let status = Command::new("mkfs")
            .args(["-F", "pcfs", "-o", "fat=32", partition.as_str()])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to format partition: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to format partition".to_string(),
            ));
        }
    }

    println!("FAT32 partition created successfully");
    Ok(())
}

/// Get the mount point of the device
fn get_mount_point(device: &str) -> Result<PathBuf, Error> {
    #[cfg(target_os = "linux")]
    {
        // Create a temporary mount point
        let mount_dir = tempdir().map_err(|e| Error::IoError(e))?;
        let mount_path = mount_dir.path().to_path_buf();

        // Mount the partition
        let partition = format!("{}p1", device);
        let status = Command::new("mount")
            .args([&partition, mount_path.to_str().unwrap()])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to mount partition: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to mount partition".to_string(),
            ));
        }

        // Return the mount point
        Ok(mount_path)
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, the partition is automatically mounted
        // Get the mount point from diskutil
        let output = Command::new(DISK_UTIL)
            .args(["info", "-plist", &format!("{}s1", device)])
            .output()
            .map_err(|e| Error::CommandError(format!("Failed to get mount point: {}", e)))?;

        // Parse the output to find the mount point
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mount_point_line = output_str
            .lines()
            .find(|line| line.contains("<key>MountPoint</key>"))
            .and_then(|_| output_str.lines().find(|line| line.contains("<string>")))
            .ok_or_else(|| Error::PartitionError("Failed to find mount point".to_string()))?;

        let mount_point = mount_point_line
            .trim()
            .trim_start_matches("<string>")
            .trim_end_matches("</string>")
            .to_string();

        Ok(PathBuf::from(mount_point))
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, we assigned drive letter Z in the diskpart script
        Ok(PathBuf::from("Z:\\"))
    }

    #[cfg(target_os = "illumos")]
    {
        // Create a temporary mount point
        let mount_dir = tempdir().map_err(|e| Error::IoError(e))?;
        let mount_path = mount_dir.path().to_path_buf();

        // Mount the partition
        let partition = format!("{}s0", device);
        let status = Command::new("mount")
            .args(["-F", "pcfs", &partition, mount_path.to_str().unwrap()])
            .status()
            .map_err(|e| Error::CommandError(format!("Failed to mount partition: {}", e)))?;

        if !status.success() {
            return Err(Error::PartitionError(
                "Failed to mount partition".to_string(),
            ));
        }

        // Return the mount point
        Ok(mount_path)
    }
}

/// Download and extract boot files from a URL
async fn download_and_extract_boot_files(url: &str, mount_point: &Path) -> Result<(), Error> {
    println!("Getting boot files from {}...", url);

    // Create a temporary directory for the download
    let temp_dir = tempdir().map_err(|e| Error::IoError(e))?;
    let archive_path = temp_dir.path().join("boot_files.tar.gz");

    // Get the file from cache or download it
    get_cached_or_download_file(url, &archive_path).await?;

    // Extract the archive
    println!("Extracting boot files...");
    extract_archive(&archive_path, mount_point)?;

    println!("Boot files extracted successfully");
    Ok(())
}

/// Get a file from the cache or download it if it's not in the cache
async fn get_cached_or_download_file(url: &str, target_path: &Path) -> Result<(), Error> {
    // Load the configuration
    let config = InstallAdmConfig::load()
        .map_err(|e| Error::CommandError(format!("Failed to load configuration: {}", e)))?;

    // Create the cache directory if it doesn't exist
    fs::create_dir_all(&config.cache_dir).map_err(|e| Error::IoError(e))?;

    // Parse the URL
    let url_obj = Url::parse(url).map_err(|e| Error::UrlParse(e))?;

    // Handle local file paths (file:// URLs)
    if url_obj.scheme() == "file" {
        let file_path = url_obj
            .to_file_path()
            .map_err(|_| Error::CommandError(format!("Invalid file URL: {}", url)))?;

        println!("Using local file: {}", file_path.display());

        // Copy the file directly to the target path
        fs::copy(&file_path, target_path).map_err(|e| Error::IoError(e))?;

        return Ok(());
    }

    // For remote URLs, use the cache
    let filename = url_obj
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("file.bin");

    let cache_path = config.cache_dir.join(filename);

    // Check if the file exists in the cache
    if cache_path.exists() {
        println!("Using cached file: {}", cache_path.display());
        // Copy the file from the cache to the target path
        fs::copy(&cache_path, target_path).map_err(|e| Error::IoError(e))?;
    } else {
        println!("Downloading file from {}...", url);
        // Download the file
        download_file(url, &cache_path).await?;
        // Copy the file from the cache to the target path
        fs::copy(&cache_path, target_path).map_err(|e| Error::IoError(e))?;
    }

    Ok(())
}

/// Download and extract additional assets
async fn download_and_extract_assets(url: &str, mount_point: &Path) -> Result<(), Error> {
    println!("Getting additional assets from {}...", url);

    // Create a temporary directory for the download
    let temp_dir = tempdir().map_err(|e| Error::IoError(e))?;
    let archive_path = temp_dir.path().join("assets.tar.gz");

    // Get the file from cache or download it
    get_cached_or_download_file(url, &archive_path).await?;

    // Extract the archive
    println!("Extracting additional assets...");
    extract_archive(&archive_path, mount_point)?;

    println!("Additional assets extracted successfully");
    Ok(())
}

/// Download an OCI image to the USB stick
async fn download_oci_image(image_ref: &str, mount_point: &Path) -> Result<(), Error> {
    println!("Downloading OCI image: {}...", image_ref);

    // Parse the image reference
    let image_reference = ImageReference::from_str(image_ref)
        .map_err(|e| Error::OciError(format!("Invalid image reference: {}", e)))?;

    // Create OCI client
    let registry_url = format!(
        "https://{}",
        image_reference
            .hostname
            .unwrap_or(String::from("localhost"))
    );
    let client = OciClient::new(registry_url, None);
    let mut session = client.new_session(image_reference.name.clone());

    // Query the manifest
    let reference = image_reference.tag.as_str();
    let manifest = session
        .query_manifest(reference)
        .await
        .map_err(|e| Error::OciError(format!("Failed to query manifest: {}", e)))?
        .ok_or_else(|| Error::OciError("Manifest not found".to_string()))?;

    // Create directory for the image
    let image_dir = mount_point
        .join("images")
        .join(image_reference.name.as_str());
    fs::create_dir_all(&image_dir).map_err(|e| Error::IoError(e))?;

    // Download layers
    match manifest {
        ManifestVariant::Manifest(image_manifest) => {
            println!("Downloading image layers...");

            // Download config
            let config_path = image_dir.join("config.json");
            session
                .download_blob(&image_manifest.config.digest, &config_path, true)
                .await
                .map_err(|e| Error::OciError(format!("Failed to download config: {}", e)))?;

            // Download layers
            for (i, layer) in image_manifest.layers.iter().enumerate() {
                println!(
                    "Downloading layer {}/{}...",
                    i + 1,
                    image_manifest.layers.len()
                );
                let layer_path = image_dir.join(format!("layer_{}.tar", i));
                session
                    .download_blob(&layer.digest, &layer_path, true)
                    .await
                    .map_err(|e| Error::OciError(format!("Failed to download layer: {}", e)))?;
            }

            // Save manifest
            let manifest_path = image_dir.join("manifest.json");
            let manifest_json =
                serde_json::to_string_pretty(&image_manifest).map_err(|e| Error::JSONError(e))?;
            fs::write(manifest_path, manifest_json).map_err(|e| Error::IoError(e))?;
        }
        _ => {
            return Err(Error::OciError(
                "Only image manifests are supported".to_string(),
            ));
        }
    }

    println!("OCI image downloaded successfully");
    Ok(())
}

/// Download a file from a URL with progress bar
async fn download_file(url: &str, path: &Path) -> Result<(), Error> {
    let client = Client::new();
    let res = client.get(url).send().await?;

    if !res.status().is_success() {
        return Err(Error::DownloadCodeError(
            path.file_name()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(String::from("unknown path")),
        ));
    }

    let total_size = res.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let mut file = File::create(path).map_err(|e| Error::IoError(e))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).map_err(|e| Error::IoError(e))?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

/// Run a testrun of the installer using libvirt
/// 
/// This function:
/// 1. Creates a bootable USB (or uses an existing one)
/// 2. Generates or copies a configuration file to the USB
/// 3. Launches a VM using libvirt to test the installation
pub async fn testrun_installer(
    device: &str,
    oci_image: Option<&str>,
    size_gb: u64,
    assets_url: Option<&str>,
    config_file: Option<&str>,
    memory_mb: u64,
    cpus: u32,
) -> Result<(), Error> {
    // Check if libvirt is available
    which("virsh").map_err(|_| Error::ToolNotFound("virsh".to_string()))?;
    which("virt-install").map_err(|_| Error::ToolNotFound("virt-install".to_string()))?;

    // Create or use existing bootable USB
    println!("Setting up bootable USB for testrun...");
    create_bootable_usb(device, oci_image, size_gb, assets_url).await?;

    // Get mount point of the USB
    let device_path = normalize_device_path(device)?;
    let (actual_device, using_loop) = ensure_device_exists(&device_path, size_gb)?;
    let mount_point = get_mount_point(&actual_device)?;


    // Generate or copy config file to USB
    if let Some(config_path) = config_file {
        println!("Copying config file to USB...");
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| Error::IoError(e))?;

        // Determine the file extension based on content or use a default
        let extension = if config_content.trim().starts_with('{') {
            // Looks like JSON
            ".json"
        } else if config_content.contains('=') && !config_content.contains(':') {
            // Likely TOML
            ".toml"
        } else if config_content.contains(':') && !config_content.contains('{') {
            // Likely YAML
            ".yaml"
        } else {
            // Default to JSON if can't determine
            ".json"
        };

        let usb_config_path = mount_point.join(format!("machined{}", extension));
        println!("Writing config file to {}", usb_config_path.display());

        let mut config_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&usb_config_path)
            .map_err(|e| Error::IoError(e))?;

        config_file.write_all(config_content.as_bytes())
            .map_err(|e| Error::IoError(e))?;
    } else {
        // Generate a default configuration file
        println!("Generating default configuration file...");

        // Generate the configuration content
        let config_content = generate_default_config(oci_image)?;

        // Write the configuration to the USB
        let usb_config_path = mount_point.join("machined.json");
        println!("Writing default config file to {}", usb_config_path.display());

        let mut config_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&usb_config_path)
            .map_err(|e| Error::IoError(e))?;

        config_file.write_all(config_content.as_bytes())
            .map_err(|e| Error::IoError(e))?;
    }

    println!("USB prepared for testrun");

    // Generate a unique VM name using timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let vm_name = format!("installer-testrun-{}", timestamp);

    // Launch VM using libvirt
    println!("Launching VM for testrun...");
    let status = Command::new("virt-install")
        .args([
            "--name", &vm_name,
            "--memory", &format!("{}", memory_mb),
            "--vcpus", &format!("{}", cpus),
            "--disk", &format!("path={},bus=usb", device_path),
            "--boot", "uefi",
            "--os-variant", "generic",
            "--graphics", "vnc",
            "--noautoconsole",
            "--import"
        ])
        .status()
        .map_err(|e| Error::CommandError(format!("Failed to launch VM: {}", e)))?;

    if !status.success() {
        return Err(Error::CommandError("Failed to launch VM".to_string()));
    }

    println!("VM launched successfully with name: {}", vm_name);
    println!("You can connect to the VM using:");
    println!("  virsh console {}", vm_name);
    println!("Or view the graphical console with:");
    println!("  virt-viewer {}", vm_name);

    // Clean up loop device if we created one
    if using_loop {
        cleanup_loop_device(&actual_device)?;
    }

    Ok(())
}

/// Generate a default configuration file for the installer
fn generate_default_config(oci_image: Option<&str>) -> Result<String, Error> {
    // Default OCI image if none provided
    let image = oci_image.unwrap_or("oci://aopc.cloud/openindiana/hipster:latest");

    // Create a default configuration
    let config = json!({
        "pool": {
            "name": "rpool",
            "vdev": {
                "kind": "Mirror",
                "disks": ["c0t0d0", "c0t1d0"]  // Default disks, will be detected by machined
            },
            "options": [
                {
                    "name": "compression",
                    "value": "zstd"
                }
            ]
        },
        "image": image,
        "boot_environment_name": "illumos",
        "sysconfig": {
            "hostname": "illumos-test",
            "nameservers": ["8.8.8.8", "8.8.4.4"],
            "interfaces": [
                {
                    "name": "net0",
                    "addresses": [
                        {
                            "name": "v4",
                            "kind": "Dhcp4"
                        }
                    ]
                }
            ]
        }
    });

    // Convert to a pretty-printed JSON string
    let config_str = serde_json::to_string_pretty(&config)
        .map_err(|e| Error::JSONError(e))?;

    Ok(config_str)
}

/// Extract an archive to a directory using libarchive
fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<(), Error> {
    // Create a new reader
    let mut reader_builder = libarchive::reader::Builder::new();

    // Set the compression based on file extension
    if archive_path.extension().map_or(false, |ext| ext == "gz") {
        reader_builder
            .support_compression(ReadCompression::Gzip)
            .map_err(|e| Error::ArchiveError(format!("Failed to set compression: {}", e)))?;
    } else {
        reader_builder
            .support_compression(ReadCompression::None)
            .map_err(|e| Error::ArchiveError(format!("Failed to set compression: {}", e)))?;
    }

    // Support common archive formats
    reader_builder
        .support_format(ReadFormat::All)
        .map_err(|e| Error::ArchiveError(format!("Failed to set format: {}", e)))?;

    // Open the archive file
    let mut reader = reader_builder
        .open_file(archive_path.to_str().ok_or(Error::ArchiveError(format!(
            "Failed to open archive: {} is not a valid path",
            archive_path.display()
        )))?)
        .map_err(|e| Error::ArchiveError(format!("Failed to open archive: {}", e)))?;

    let writer = libarchive::writer::Disk::new();
    writer
        .write(&mut reader, target_dir.as_os_str().to_str())
        .map_err(|e| Error::ArchiveError(format!("Failed to extract archive: {}", e)))?;

    Ok(())
}

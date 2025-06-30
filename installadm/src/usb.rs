use anyhow::Result;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use ociclient::{Client as OciClient, ImageReference, ManifestVariant};
use reqwest::Client;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use tar::Archive;
use tempfile::tempdir;
use which::which;

use crate::Error;

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
    boot_files_url: &str,
    oci_image: Option<&str>,
    size_gb: u64,
    assets_url: Option<&str>,
) -> Result<(), Error> {
    // Check if required tools are available
    check_required_tools()?;

    // Normalize device path
    let device_path = normalize_device_path(device)?;
    println!("Creating bootable USB on device: {}", device_path);

    // Ensure the device exists, creating an empty file if it doesn't
    // Use the specified size or a default size if creating a new file
    ensure_device_exists(&device_path, size_gb)?;

    // Create FAT32 partition
    create_fat32_partition(&device_path, size_gb)?;

    // Get mount point
    let mount_point = get_mount_point(&device_path)?;
    println!("USB device mounted at: {}", mount_point.display());

    // Download and extract boot files
    download_and_extract_boot_files(boot_files_url, &mount_point).await?;

    // Download additional assets if specified
    if let Some(url) = assets_url {
        download_and_extract_assets(url, &mount_point).await?;
    }

    // Download OCI image if specified
    if let Some(image) = oci_image {
        download_oci_image(image, &mount_point).await?;
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
    }

    #[cfg(target_os = "macos")]
    {
        which("newfs_msdos").map_err(|_| Error::ToolNotFound("newfs_msdos".to_string()))?;
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
    }

    Ok(())
}

/// Normalize device path based on platform
fn normalize_device_path(device: &str) -> Result<String, Error> {
    if device.starts_with(DEVICE_PREFIX) {
        Ok(device.to_string())
    } else {
        Ok(format!("{}{}", DEVICE_PREFIX, device))
    }
}

/// Ensure the device exists, creating an empty file if it doesn't
fn ensure_device_exists(device_path: &str, size_gb: u64) -> Result<(), Error> {
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

    Ok(())
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
                &format!("{}GiB", size_gb),
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
        let partition = format!("{}1", device);
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
        let partition = format!("{}1", device);
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
    println!("Downloading boot files from {}...", url);

    // Create a temporary directory for the download
    let temp_dir = tempdir().map_err(|e| Error::IoError(e))?;
    let archive_path = temp_dir.path().join("boot_files.tar.gz");

    // Download the file
    download_file(url, &archive_path).await?;

    // Extract the archive
    println!("Extracting boot files...");
    extract_archive(&archive_path, mount_point)?;

    println!("Boot files extracted successfully");
    Ok(())
}

/// Download and extract additional assets
async fn download_and_extract_assets(url: &str, mount_point: &Path) -> Result<(), Error> {
    println!("Downloading additional assets from {}...", url);

    // Create a temporary directory for the download
    let temp_dir = tempdir().map_err(|e| Error::IoError(e))?;
    let archive_path = temp_dir.path().join("assets.tar.gz");

    // Download the file
    download_file(url, &archive_path).await?;

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

/// Extract a tar archive to a directory
fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<(), Error> {
    let file = File::open(archive_path).map_err(|e| Error::IoError(e))?;

    // Check if it's a gzipped archive
    if archive_path.extension().map_or(false, |ext| ext == "gz") {
        let decompressed = flate2::read::GzDecoder::new(file);
        let mut archive = Archive::new(decompressed);
        archive
            .unpack(target_dir)
            .map_err(|e| Error::ArchiveError(format!("Failed to extract archive: {}", e)))?;
    } else {
        let mut archive = Archive::new(file);
        archive
            .unpack(target_dir)
            .map_err(|e| Error::ArchiveError(format!("Failed to extract archive: {}", e)))?;
    }

    Ok(())
}

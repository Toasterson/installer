use crate::machined::claim_request::ClaimSecret;
use crate::machined::machine_service_client::MachineServiceClient;
use crate::machined::{ClaimRequest, InstallConfig, SystemInfoRequest};
use crate::state::{read_state_file, save_state, Server};
use clap::{Parser, Subcommand};
use miette::Diagnostic;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use tonic::codec::CompressionEncoding;
use tonic::codegen::http;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Status;
use url::Url;

mod config;
mod machined;
mod state;
mod usb;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("server responded with error: {0}")]
    ServerError(#[from] Status),
    #[error(transparent)]
    TransportError(#[from] tonic::transport::Error),
    #[error(transparent)]
    URLConversionError(#[from] http::uri::InvalidUri),
    #[error("No App Directory")]
    NoAppDir,
    #[error("Currently only password claims are supported")]
    CurrentlyPasswordClaimRequired,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    JSONError(#[from] serde_json::Error),
    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
    #[error("No such server please claim it first")]
    NoSuchServer,
    #[error("No parent dir")]
    NoParentDir,
    #[error("Please provide a servername none can be inferred")]
    ServerNameCannotBeInferred,
    #[error("Failed to create FAT32 partition: {0}")]
    PartitionError(String),
    #[error("Failed to download file: {0}")]
    DownloadError(#[from] reqwest::Error),
    #[error("Failed to download file: {0}")]
    DownloadCodeError(String),
    #[error("Failed to extract archive: {0}")]
    ArchiveError(String),
    #[error("Failed to find required tool: {0}")]
    ToolNotFound(String),
    #[error("Failed to execute command: {0}")]
    CommandError(String),
    #[error("Failed to download OCI image: {0}")]
    OciError(String),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Claim {
        url: String,
        secret: Option<String>,
        name: Option<String>,
    },
    Install {
        name: String,
        #[arg(short, long)]
        config: PathBuf,
    },
    /// Retrieve system information from a machined server
    ///
    /// This command connects to a machined server and retrieves information about
    /// the system, including disk and network interface details.
    SystemInfo {
        /// Name of the server to connect to
        name: String,
    },
    /// Create a bootable USB stick with EFI boot files
    ///
    /// The boot files URL is configured in one of the following locations:
    /// - /etc/installadm/config
    /// - /etc/installadm/config.<RUN_MODE>
    /// - ~/.config/installadm/config
    /// - Environment variable INSTALLADM_BOOT_FILES_URL
    CreateBootableUsb {
        /// Path to the USB device (e.g., /dev/sdb on Linux, disk2 on macOS) or an image file
        /// If an image file is provided, it will be mounted as a loop device
        device: String,

        /// Optional OCI image to download to the USB stick
        #[arg(short, long)]
        oci_image: Option<String>,

        /// Optional size of the FAT32 partition in GB (default: 4)
        #[arg(short, long, default_value = "4")]
        size: u64,

        /// Optional additional assets URL
        #[arg(short, long)]
        assets_url: Option<String>,
    },
    /// Test run the installer using libvirt
    ///
    /// Creates a bootable USB and launches a VM to test the installation
    /// Currently only supports Linux with libvirt
    TestrunInstaller {
        /// Path to the USB device (e.g., /dev/sdb on Linux) or an image file
        /// If an image file is provided, it will be mounted as a loop device
        device: String,

        /// Optional OCI image to download to the USB stick
        #[arg(short, long)]
        oci_image: Option<String>,

        /// Optional size of the FAT32 partition in GB (default: 4)
        #[arg(short, long, default_value = "4")]
        size: u64,

        /// Optional additional assets URL
        #[arg(short, long)]
        assets_url: Option<String>,

        /// Optional configuration file to use for installation
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Memory size for the VM in MB (default: 2048)
        #[arg(short, long, default_value = "2048")]
        memory: u64,

        /// Number of CPUs for the VM (default: 2)
        #[arg(short, long, default_value = "2")]
        cpus: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut state = read_state_file()?;

    match args.command {
        Commands::Claim { secret, url, name } => {
            let url_url: Url = url.parse()?;
            if secret.is_none() {
                return Err(Error::CurrentlyPasswordClaimRequired);
            }

            let server_name = if let Some(arg_name) = name {
                arg_name
            } else if let Some(host_name_str) = url_url.host_str() {
                host_name_str.to_owned()
            } else {
                return Err(Error::ServerNameCannotBeInferred);
            };

            let claim_request = tonic::Request::new(ClaimRequest {
                claim_secret: secret.map(|s| ClaimSecret::ClaimPassword(s)),
            });
            let mut client = connect(url.as_str()).await?;

            let response = client.claim(claim_request).await?;
            let claim_response = response.into_inner();
            let srv = Server {
                name: server_name.clone(),
                uri: url,
                claim_token: claim_response.claim_token,
            };
            state.add_server(srv);
            save_state(state)?;
        }
        Commands::Install { config, name } => {
            let server = state.get_server(&name).ok_or(Error::NoSuchServer)?;
            let machineconfig = read_to_string(&config)?;
            let install_request = tonic::Request::new(InstallConfig { machineconfig });
            let mut client = connect(server.uri.as_str()).await?;
            let response = client.install(install_request).await?;
            let mut stream = response.into_inner();
            while let Some(stream_resp) = stream.next().await {
                let progress = stream_resp?;
                println!("{}: {:?}", progress.level, progress.message);
            }
        }
        Commands::SystemInfo { name } => {
            let server = state.get_server(&name).ok_or(Error::NoSuchServer)?;
            let mut client = connect(server.uri.as_str()).await?;
            
            println!("Retrieving system information from server: {}", name);
            let request = tonic::Request::new(SystemInfoRequest {});
            let response = client.get_system_info(request).await?;
            let system_info = response.into_inner();
            
            // Display disk information
            println!("\nDisk Information:");
            println!("{:<10} {:<15} {:<20} {:<15} {:<10} {:<10} {:<15} {:<5} {:<5} {:<10}", 
                     "Device", "Vendor", "Product", "Size", "Removable", "SSD", "Serial", "FLT", "LOC", "Location");
            println!("{:-<120}", "");
            
            for disk in system_info.disks {
                // Convert size to human-readable format
                let size = format_size(disk.size_bytes);
                
                println!("{:<10} {:<15} {:<20} {:<15} {:<10} {:<10} {:<15} {:<5} {:<5} {:<10}", 
                         disk.device, 
                         disk.vendor, 
                         disk.product, 
                         size,
                         if disk.removable { "Yes" } else { "No" },
                         if disk.solid_state { "Yes" } else { "No" },
                         disk.serial,
                         disk.fault_status,
                         disk.location_code,
                         disk.chassis_bay);
            }
            
            // Display network interface information
            println!("\nNetwork Interface Information:");
            println!("{:<10} {:<10} {:<10} {:<10} {:<15} {:<10} {:<20}", 
                     "Name", "Class", "Media", "State", "Speed", "MTU", "MAC Address");
            println!("{:-<100}", "");
            
            for interface in system_info.network_interfaces {
                println!("{:<10} {:<10} {:<10} {:<10} {:<15} {:<10} {:<20}", 
                         interface.name, 
                         interface.class, 
                         interface.media, 
                         interface.state, 
                         interface.speed,
                         interface.mtu,
                         interface.mac_address);
            }
            
            // Display SMBIOS information if available
            if let Some(smbios) = &system_info.smbios {
                // Display BIOS information
                if let Some(bios) = &smbios.bios {
                    println!("\nBIOS Information:");
                    println!("Vendor: {}", bios.vendor);
                    println!("Version: {}", bios.version);
                    println!("Release Date: {}", bios.release_date);
                    println!("Address Segment: {}", bios.address_segment);
                    println!("ROM Size: {} bytes", bios.rom_size);
                    println!("Image Size: {} bytes", bios.image_size);
                    println!("Characteristics: 0x{:x}", bios.characteristics);
                    println!("Characteristics Extension Byte 1: 0x{:x}", bios.characteristics_ext1);
                    println!("Characteristics Extension Byte 2: 0x{:x}", bios.characteristics_ext2);
                    println!("Version Number: {}", bios.version_number);
                }
                
                // Display System information
                if let Some(system) = &smbios.system {
                    println!("\nSystem Information:");
                    println!("Manufacturer: {}", system.manufacturer);
                    println!("Product: {}", system.product);
                    println!("Version: {}", system.version);
                    println!("Serial Number: {}", system.serial_number);
                    println!("UUID: {}", system.uuid);
                    println!("Wake-Up Event: 0x{:x}", system.wakeup_event);
                    println!("SKU Number: {}", system.sku_number);
                    println!("Family: {}", system.family);
                }
                
                // Display Baseboard information
                if let Some(baseboard) = &smbios.baseboard {
                    println!("\nBaseboard Information:");
                    println!("Manufacturer: {}", baseboard.manufacturer);
                    println!("Product: {}", baseboard.product);
                    println!("Version: {}", baseboard.version);
                    println!("Serial Number: {}", baseboard.serial_number);
                    println!("Asset Tag: {}", baseboard.asset_tag);
                    println!("Location Tag: {}", baseboard.location_tag);
                    println!("Chassis: {}", baseboard.chassis);
                    println!("Flags: 0x{:x}", baseboard.flags);
                    println!("Board Type: 0x{:x}", baseboard.board_type);
                }
                
                // Display Chassis information
                if let Some(chassis) = &smbios.chassis {
                    println!("\nChassis Information:");
                    println!("Manufacturer: {}", chassis.manufacturer);
                    println!("Version: {}", chassis.version);
                    println!("Serial Number: {}", chassis.serial_number);
                    println!("Asset Tag: {}", chassis.asset_tag);
                    println!("OEM Data: 0x{:x}", chassis.oem_data);
                    println!("SKU Number: {}", chassis.sku_number);
                    println!("Lock Present: {}", if chassis.lock_present { "Yes" } else { "No" });
                    println!("Chassis Type: 0x{:x}", chassis.chassis_type);
                    println!("Boot-Up State: 0x{:x}", chassis.boot_up_state);
                    println!("Power Supply State: 0x{:x}", chassis.power_supply_state);
                    println!("Thermal State: 0x{:x}", chassis.thermal_state);
                    println!("Chassis Height: {}u", chassis.chassis_height);
                    println!("Power Cords: {}", chassis.power_cords);
                    println!("Element Records: {}", chassis.element_records);
                }
                
                // Display Processor information
                if !smbios.processors.is_empty() {
                    println!("\nProcessor Information:");
                    for (i, processor) in smbios.processors.iter().enumerate() {
                        println!("Processor #{}", i);
                        println!("  Manufacturer: {}", processor.manufacturer);
                        println!("  Version: {}", processor.version);
                        println!("  Serial Number: {}", processor.serial_number);
                        println!("  Asset Tag: {}", processor.asset_tag);
                        println!("  Location Tag: {}", processor.location_tag);
                        println!("  Part Number: {}", processor.part_number);
                        println!("  Family: {}", processor.family);
                        println!("  CPUID: 0x{:x}", processor.cpuid);
                        println!("  Type: {}", processor.r#type);
                        println!("  Socket Upgrade: {}", processor.socket_upgrade);
                        println!("  Socket Populated: {}", if processor.socket_populated { "Yes" } else { "No" });
                        println!("  Processor Status: {}", processor.processor_status);
                        println!("  Supported Voltages: {}", processor.supported_voltages);
                        println!("  Core Count: {}", processor.core_count);
                        println!("  Cores Enabled: {}", processor.cores_enabled);
                        println!("  Thread Count: {}", processor.thread_count);
                        println!("  Processor Characteristics: 0x{:x}", processor.processor_characteristics);
                        println!("  External Clock Speed: {}", processor.external_clock);
                        println!("  Maximum Speed: {}", processor.maximum_speed);
                        println!("  Current Speed: {}", processor.current_speed);
                        println!("  L1 Cache Handle: {}", processor.l1_cache_handle);
                        println!("  L2 Cache Handle: {}", processor.l2_cache_handle);
                        println!("  L3 Cache Handle: {}", processor.l3_cache_handle);
                        println!("  Threads Enabled: {}", processor.threads_enabled);
                    }
                }
                
                // Display Memory Array information
                if !smbios.memory_arrays.is_empty() {
                    println!("\nMemory Array Information:");
                    for (i, memory_array) in smbios.memory_arrays.iter().enumerate() {
                        println!("Memory Array #{}", i);
                        println!("  Location: {}", memory_array.location);
                        println!("  Use: {}", memory_array.r#use);
                        println!("  ECC: {}", memory_array.ecc);
                        println!("  Number of Slots/Sockets: {}", memory_array.slots);
                        println!("  Max Capacity: {} bytes", memory_array.max_capacity);
                    }
                }
                
                // Display Memory Device information
                if !smbios.memory_devices.is_empty() {
                    println!("\nMemory Device Information:");
                    for (i, memory_device) in smbios.memory_devices.iter().enumerate() {
                        println!("Memory Device #{}", i);
                        println!("  Manufacturer: {}", memory_device.manufacturer);
                        println!("  Serial Number: {}", memory_device.serial_number);
                        println!("  Asset Tag: {}", memory_device.asset_tag);
                        println!("  Location Tag: {}", memory_device.location_tag);
                        println!("  Part Number: {}", memory_device.part_number);
                        println!("  Physical Memory Array: {}", memory_device.array_handle);
                        println!("  Memory Error Data: {}", memory_device.error_handle);
                        println!("  Total Width: {} bits", memory_device.total_width);
                        println!("  Data Width: {} bits", memory_device.data_width);
                        println!("  Size: {} bytes", memory_device.size);
                        println!("  Form Factor: {}", memory_device.form_factor);
                        println!("  Set: {}", memory_device.set);
                        println!("  Rank: {}", memory_device.rank);
                        println!("  Memory Type: {}", memory_device.memory_type);
                        println!("  Flags: 0x{:x}", memory_device.flags);
                        println!("  Speed: {}", memory_device.speed);
                        println!("  Configured Speed: {}", memory_device.configured_speed);
                        println!("  Device Locator: {}", memory_device.device_locator);
                        println!("  Bank Locator: {}", memory_device.bank_locator);
                        println!("  Minimum Voltage: {}", memory_device.min_voltage);
                        println!("  Maximum Voltage: {}", memory_device.max_voltage);
                        println!("  Configured Voltage: {}", memory_device.configured_voltage);
                    }
                }
                
                // Display Memory Array Mapped Address information
                if !smbios.memory_array_mapped_addresses.is_empty() {
                    println!("\nMemory Array Mapped Address Information:");
                    for (i, memory_array_mapped_address) in smbios.memory_array_mapped_addresses.iter().enumerate() {
                        println!("Memory Array Mapped Address #{}", i);
                        println!("  Physical Memory Array: {}", memory_array_mapped_address.array_handle);
                        println!("  Devices per Row: {}", memory_array_mapped_address.devices_per_row);
                        println!("  Physical Address: 0x{:x}", memory_array_mapped_address.physical_address);
                        println!("  Size: {} bytes", memory_array_mapped_address.size);
                    }
                }
                
                // Display Boot information
                if let Some(boot) = &smbios.boot {
                    println!("\nSystem Boot Information:");
                    println!("Boot Status Code: 0x{:x}", boot.status_code);
                }
            }
        }
        Commands::CreateBootableUsb {
            device,
            oci_image,
            size,
            assets_url,
        } => {
            println!("Creating bootable USB stick on device: {}", device);
            usb::create_bootable_usb(&device, oci_image.as_deref(), size, assets_url.as_deref())
                .await?;
        },
        Commands::TestrunInstaller {
            device,
            oci_image,
            size,
            assets_url,
            config,
            memory,
            cpus,
        } => {
            println!("Test running installer on device: {}", device);
            usb::testrun_installer(
                &device,
                oci_image.as_deref(),
                size,
                assets_url.as_deref(),
                config.as_deref().map(|p| p.to_str().unwrap_or_default()),
                memory,
                cpus,
            )
            .await?;
        }
    }

    Ok(())
}

async fn connect(url: &str) -> Result<MachineServiceClient<Channel>> {
    let channel = Channel::builder(http::Uri::from_str(url)?)
        .connect()
        .await?;

    let client = MachineServiceClient::new(channel)
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd);
    Ok(client)
}

/// Format a size in bytes to a human-readable string
fn format_size(size_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size_bytes < KB {
        format!("{} B", size_bytes)
    } else if size_bytes < MB {
        format!("{:.1} KB", size_bytes as f64 / KB as f64)
    } else if size_bytes < GB {
        format!("{:.1} MB", size_bytes as f64 / MB as f64)
    } else if size_bytes < TB {
        format!("{:.1} GB", size_bytes as f64 / GB as f64)
    } else {
        format!("{:.1} TB", size_bytes as f64 / TB as f64)
    }
}
